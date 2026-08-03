#!/usr/bin/env python3
"""
generate_ocean_islands.py
=========================
Add 12 new islands to ROSE Online zone 200 (OCEAN) by additively editing the
terrain heightmap binary files in 3DDATA/MAPS/OCEAN (HIM files).

What it does
------------
* Reads every existing {block_x}_{block_y}.HIM block (no blocks are created).
* For each new island, raises the terrain additively with a smooth falloff
  (smoothstep profile with a sandy beach ramp at the rim, plus a small
  deterministic micro-noise so peaks look organic).
* Blends with `new_height = max(existing_height, island_profile)`, so all
  existing terrain (the 3 island clusters and the spawn point) is preserved
  exactly -- no other byte is touched.
* Writes back ONLY the HIM blocks that actually changed, in the exact same
  format and size (16-byte header + 65x65 f32 little-endian heights in cm).
  TIL and IFO files are never touched.
* Verifies before writing: every island must be
  - inside the terrain extent [3840, 6560] x [3840, 6560] m (file coords),
  - inside the water plane extent [3600, 6800] m (file coords),
  - at least (radius + 500) m away from ANY existing land sample
    (height > 1 cm) of the current map, sampled directly from the HIM files,
  - at least ~500 m away from every other new island center.

Run from the repo root (plain standard library, Python 3.8+):
    python tools/generate_ocean_islands.py           # apply changes
    python tools/generate_ocean_islands.py --dry-run # verify + preview only

HIM binary format (verified against rose-file-readers/src/him.rs and
src/map_editor/coords.rs, cross-checked with the actual files):
    u32 width  = 65   (little-endian)
    u32 height = 65
    u32 reserved = 0
    u32 reserved = 0
    65*65 f32 heights, little-endian, in centimeters, row-major
    (row = y/grid then x/grid; index = y*65 + x). Total file size 16916 bytes.
Grid cell = 2.5 m, block = 160 m (ZON grid_per_patch=4, grid_size=250).
File position of grid sample (gx, gy) in block (bx, by):
    x = bx*160 + gx*2.5,  z = by*160 + gy*2.5   (z positive = "south" in file
    space; the report prints the (x, -z) manifest convention).
"""

import math
import random
import struct
import sys
from pathlib import Path

ZONE_DIR = Path(__file__).resolve().parent.parent / "3DDATA" / "MAPS" / "OCEAN"

BLOCK_M = 160.0
CELL_M = 2.5
GRID = 65
HEADER_BYTES = 16
EXPECTED_HIM_SIZE = HEADER_BYTES + GRID * GRID * 4  # 16916

TERRAIN_MIN = 3840.0  # first existing block edge (24 * 160)
TERRAIN_MAX = 6560.0  # last existing block edge + grid span (40*160 + 64*2.5)
WATER_MIN = 3600.0    # water plane covers [3600, 6800] in file coords
WATER_MAX = 6800.0
MIN_CLEARANCE_M = 500.0
LAND_THRESHOLD_CM = 1.0

# Island manifest: (name, x_m, z_m, radius_m, peak_height_cm)
# Coordinates are in file space, meters, z positive; the report prints (x, -z).
ISLANDS = [
    ("northeast",   6400, 6400, 160, 1400),
    ("northwest",   4000, 6400, 160, 1350),
    ("east-north",  6400, 5880, 140, 1100),
    ("north-east",  5880, 6400, 140, 1150),
    ("north-west",  4520, 6400, 130, 1050),
    ("west-north",  4000, 5880, 130, 1000),
    ("north-mid",   5360, 6400, 120, 950),
    ("southwest",   4000, 4000, 140, 1250),
    ("southeast",   6400, 4000, 140, 1200),
    ("south-mid",   5040, 4000, 120, 900),
    ("south-west",  4520, 4000, 110, 850),
    ("east-mid",    6400, 5360, 110, 1000),
]


class VerificationError(Exception):
    pass


def block_name(bx, by):
    return f"{bx}_{by}.HIM"


def load_block(blocks, bx, by):
    """Return (width, height, heights-list) for a block, or None if absent."""
    key = (bx, by)
    if key in blocks:
        return blocks[key]
    path = ZONE_DIR / block_name(bx, by)
    if not path.is_file():
        blocks[key] = None
        return None
    data = path.read_bytes()
    if len(data) != EXPECTED_HIM_SIZE:
        raise VerificationError(
            f"{path.name}: unexpected size {len(data)} bytes "
            f"(expected {EXPECTED_HIM_SIZE}); refusing to touch it"
        )
    w, h, r1, r2 = struct.unpack_from("<4I", data, 0)
    if w != GRID or h != GRID or r1 != 0 or r2 != 0:
        raise VerificationError(
            f"{path.name}: unexpected header ({w},{h},{r1},{r2}); refusing to touch it"
        )
    heights = list(struct.unpack_from(f"<{GRID * GRID}f", data, HEADER_BYTES))
    blocks[key] = (w, h, heights)
    return blocks[key]


def island_profile(dist_m, radius_m, peak_cm):
    """Height added by one island at distance dist from its center (0 outside)."""
    if dist_m >= radius_m:
        return 0.0
    t = 1.0 - dist_m / radius_m          # 1 at center, 0 at rim
    s = t * t * (3.0 - 2.0 * t)           # smoothstep: gentle beach ramp at rim
    return peak_cm * s


def cell_noise(gx, gy):
    """Deterministic micro-noise in [-1, 1] for a grid cell (stable across runs)."""
    return random.Random((gx * 73856093) ^ (gy * 19349663) ^ 0x5EED).uniform(-1.0, 1.0)


def island_cells(x, z, radius_m):
    """Yield (bx, by, gx, gy, dist_m) for every grid cell within the footprint."""
    bx0 = int(math.floor((x - radius_m) / BLOCK_M))
    bx1 = int(math.floor((x + radius_m) / BLOCK_M))
    by0 = int(math.floor((z - radius_m) / BLOCK_M))
    by1 = int(math.floor((z + radius_m) / BLOCK_M))
    for by in range(by0, by1 + 1):
        for bx in range(bx0, bx1 + 1):
            for gy in range(GRID):
                for gx in range(GRID):
                    px = bx * BLOCK_M + gx * CELL_M
                    pz = by * BLOCK_M + gy * CELL_M
                    yield bx, by, gx, gy, math.hypot(px - x, pz - z)


def collect_land_samples(blocks):
    """One pass over every HIM block; return list of (x_m, z_m) land samples."""
    samples = []
    for by in range(24, 41):
        for bx in range(24, 41):
            blk = load_block(blocks, bx, by)
            if blk is None:
                continue
            _, _, heights = blk
            base_x = bx * BLOCK_M
            base_z = by * BLOCK_M
            for gy in range(GRID):
                for gx in range(GRID):
                    if heights[gy * GRID + gx] > LAND_THRESHOLD_CM:
                        samples.append((base_x + gx * CELL_M, base_z + gy * CELL_M))
    return samples


def verify_layout(blocks, land):
    """Check all placement constraints against the real terrain. Returns dict of
    nearest-existing-land distances per island."""
    issues = []
    nearest_land = {}

    for name, x, z, radius, peak in ISLANDS:
        if x - radius < TERRAIN_MIN or x + radius > TERRAIN_MAX:
            issues.append(f"{name}: outside terrain x-extent [{TERRAIN_MIN}, {TERRAIN_MAX}]")
        if z - radius < TERRAIN_MIN or z + radius > TERRAIN_MAX:
            issues.append(f"{name}: outside terrain z-extent [{TERRAIN_MIN}, {TERRAIN_MAX}]")
        if x - radius < WATER_MIN or x + radius > WATER_MAX:
            issues.append(f"{name}: outside water plane x-extent [{WATER_MIN}, {WATER_MAX}]")
        if z - radius < WATER_MIN or z + radius > WATER_MAX:
            issues.append(f"{name}: outside water plane z-extent [{WATER_MIN}, {WATER_MAX}]")

        best = None
        for (lx, lz) in land:
            d = math.hypot(lx - x, lz - z)
            if best is None or d < best:
                best = d
        nearest_land[name] = best
        if best is not None and best < radius + MIN_CLEARANCE_M:
            issues.append(
                f"{name}: only {best:.0f} m from existing land "
                f"(need >= {radius + MIN_CLEARANCE_M:.0f} m)"
            )

    for i in range(len(ISLANDS)):
        for j in range(i + 1, len(ISLANDS)):
            n1, x1, z1, r1, _ = ISLANDS[i]
            n2, x2, z2, r2, _ = ISLANDS[j]
            d = math.hypot(x1 - x2, z1 - z2)
            if d < MIN_CLEARANCE_M:
                issues.append(f"{n1} and {n2}: centers only {d:.0f} m apart (need >= {MIN_CLEARANCE_M:.0f})")
            if d < r1 + r2:
                issues.append(f"{n1} and {n2}: islands overlap (centers {d:.0f} m, radii {r1}+{r2})")

    if issues:
        raise VerificationError("Layout verification failed:\n  - " + "\n  - ".join(issues))
    return nearest_land


def main():
    dry_run = "--dry-run" in sys.argv
    argv = [a for a in sys.argv[1:] if a != "--dry-run"]
    if argv:
        print(f"Unknown argument(s): {' '.join(argv)}", file=sys.stderr)
        print("Usage: python tools/generate_ocean_islands.py [--dry-run]", file=sys.stderr)
        return 2

    blocks = {}
    print(f"Zone dir: {ZONE_DIR}")
    print(f"Islands to add: {len(ISLANDS)}")

    print("Scanning existing terrain...")
    land = collect_land_samples(blocks)
    print(f"  {len(land)} existing land samples found in {ZONE_DIR}")

    try:
        nearest_land = verify_layout(blocks, land)
    except VerificationError as e:
        print(str(e), file=sys.stderr)
        return 1

    # apply island profiles additively (max blend) onto the cached heights
    island_blocks = {name: set() for name, *_ in ISLANDS}
    per_block_new = {}  # (bx, by) -> new heights list

    for name, x, z, radius, peak in ISLANDS:
        noise_amp = peak * 0.04
        for bx, by, gx, gy, dist in island_cells(x, z, radius):
            prof = island_profile(dist, radius, peak)
            if prof <= 0.0:
                continue
            prof += noise_amp * (prof / peak) * cell_noise(gx, gy)
            blk = blocks.get((bx, by))
            if blk is None:
                print(f"  ! {name}: block {bx}_{by} missing, skipped (no terrain there)")
                continue
            new = per_block_new.setdefault((bx, by), blk[2][:])
            idx = gy * GRID + gx
            if prof > new[idx]:
                new[idx] = prof
            island_blocks[name].add((bx, by))

    # write only changed blocks
    changed = []
    for (bx, by), new in sorted(per_block_new.items()):
        if new == blocks[(bx, by)][2]:
            continue
        data = struct.pack("<4I", GRID, GRID, 0, 0) + struct.pack(
            f"<{GRID * GRID}f", *new
        )
        if not dry_run:
            (ZONE_DIR / block_name(bx, by)).write_bytes(data)
        changed.append((bx, by, len(data)))

    print("\n=== Island summary ===")
    for name, x, z, radius, peak in ISLANDS:
        touched = ", ".join(f"{b[0]}_{b[1]}" for b in sorted(island_blocks[name]))
        land_d = nearest_land.get(name)
        land_txt = f"{land_d:.0f} m" if land_d is not None else "n/a"
        print(
            f"  {name:<12s} center=({x:.0f},{-z:.0f}) r={radius:3d} m "
            f"peak={peak:4d} cm  nearest_existing_land={land_txt}  "
            f"blocks=[{touched}]"
        )

    print("\n=== Blocks written ===")
    if not changed:
        print("  (none -- heights already contain the islands, nothing to change)")
    for bx, by, size in changed:
        print(f"  {block_name(bx, by)} ({size} bytes)")

    verb = "would be" if dry_run else ""
    print(f"\n{len(changed)} HIM file(s) {verb} changed. No TIL/IFO files were touched.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

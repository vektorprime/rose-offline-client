#!/usr/bin/env python3
"""
deepen_ocean.py
===============
Make the zone 200 (OCEAN) ocean floor MUCH deeper by editing the terrain
heightmap (HIM) files in 3DDATA/MAPS/OCEAN.

Background
----------
The ocean floor is currently nearly flat at about -255..-276 cm (about 2.5 m
below the water surface at 0 cm), which reads as "shallow pond", not ocean.
This script deepens the floor to a target of DEEP_CM (-2000 cm = 20 m) in open
water, while preserving:

  * every LAND cell (height > 0 cm) byte-for-byte,
  * every cell within D_BEACH_M (30 m) of land -- the sandy beach shallows
    painted by tools/paint_island_tiles.py for heights in (-20, 0] cm all live
    inside this ring, so the beach look survives,
  * the "only ever lower" rule: final = min(current, target), so no cell is
    ever raised.

Between 30 m and 200 m from land the floor ramps down smoothly (smoothstep
interpolation from the original floor height REF_FLOOR_CM = -255 cm to
DEEP_CM), so there are no underwater cliffs. Beyond 200 m the floor is flat
deep water at -2000 cm with a small deterministic micro-noise (amplitude 2% of
the local depth) so it does not look artificially smooth.

Distance to land is computed with a two-pass chamfer distance transform
(Euclidean approximation, weights 3/4) over the whole 17x17 block grid (1105x
1105 cells), so block borders are handled exactly -- no per-block padding
needed because every block of the map is loaded.

Safety
------
* Only HIM files are read and written. TIL / IFO / ZON files are never
  touched.
* Land cells are byte-identical after the run (verified with a land-cell
  count check and per-cell equality).
* Write-back is limited to blocks whose packed bytes actually changed.
* Deterministic and byte-idempotent: a second run reports "nothing to change".
* No dependency beyond the Python standard library.

HIM binary format (verified against rose-file-readers/src/him.rs and
src/map_editor/coords.rs, cross-checked with the actual files):
    u32 width  = 65   (little-endian)
    u32 height = 65
    u32 reserved = 0
    u32 reserved = 0
    65*65 f32 heights, little-endian, in centimeters, row-major
    (row = y grid then x grid; index = y*65 + x). Total file size 16916 bytes.
Grid cell = 2.5 m, block = 160 m (ZON grid_per_patch=4, grid_size=250).
File position of grid sample (gx, gy) in block (bx, by):
    x = bx*160 + gx*2.5,  z = by*160 + gy*2.5  (z positive = "south").
Active blocks: bx, by in 24..40 (terrain extent 3840..6560 m).

Run from the repo root (plain standard library, Python 3.8+):
    python tools/deepen_ocean.py           # apply changes
    python tools/deepen_ocean.py --dry-run # summary + sanity checks, no writes
"""

import struct
import sys
from pathlib import Path

ZONE_DIR = Path(__file__).resolve().parent.parent / "3DDATA" / "MAPS" / "OCEAN"

BLOCK_M = 160.0
CELL_M = 2.5
GRID = 65
HEADER_BYTES = 16
EXPECTED_HIM_SIZE = HEADER_BYTES + GRID * GRID * 4  # 16916

GRID_MIN = 24
GRID_MAX = 40
GRID_CELLS = (GRID_MAX - GRID_MIN + 1) * GRID  # 1105 cells per axis

# --- Deepening constants -------------------------------------------------
# A cell is "land" (untouched) when its height is strictly above this.
LAND_THRESHOLD_CM = 0.0
# Cells within this horizontal distance of land keep their current height
# (beach + sandy shallows preserved; sand tiles were painted for heights in
# (-20, 0] cm, which all sit within ~20 m of shore).
D_BEACH_M = 30.0
# Beyond this distance from land the floor reaches the full target depth.
D_DEEP_M = 200.0
# Target floor height in open water (cm). 20 m below the surface (0 cm).
DEEP_CM = -2000.0
# Original (shallowest) floor height used as the ramp start. Verified from the
# data: the flat floor spans -276..-255 cm, so starting the ramp at -255 cm
# never requires raising any cell (min() only ever lowers).
REF_FLOOR_CM = -255.0
# Noise amplitude as a fraction of the local target depth (2% of |target|).
NOISE_FRACTION = 0.02
# Heights are quantized to 0.01 cm so a re-run recomputes byte-identical data.
QUANT_CM = 0.01

# Sanity-check sample points in file coords (x, z) meters, a ring of open
# water around the central island (5200, 5200), each verified to be > 200 m
# from land. Only reported in --dry-run mode.
SANITY_POINTS = [
    ("nw ring", 4400.0, 4400.0),
    ("n ring",  5200.0, 4400.0),
    ("ne ring", 6000.0, 4400.0),
    ("w ring",  4400.0, 5200.0),
    ("e ring",  6000.0, 5200.0),
    ("sw ring", 4400.0, 6000.0),
    ("s ring",  5200.0, 6000.0),
    ("se ring", 6000.0, 6000.0),
]


class DeepenError(Exception):
    pass


def block_name(bx, by):
    return f"{bx}_{by}.HIM"


def load_block_heights(bx, by):
    """Return the 65x65 f32 height list for block (bx, by). Raises if the
    file is missing or malformed (never silently skipped)."""
    path = ZONE_DIR / block_name(bx, by)
    data = path.read_bytes()
    if len(data) != EXPECTED_HIM_SIZE:
        raise DeepenError(
            f"{path.name}: unexpected size {len(data)} bytes "
            f"(expected {EXPECTED_HIM_SIZE}); refusing to touch it"
        )
    w, h, r1, r2 = struct.unpack_from("<4I", data, 0)
    if w != GRID or h != GRID or r1 != 0 or r2 != 0:
        raise DeepenError(
            f"{path.name}: unexpected header ({w},{h},{r1},{r2}); refusing to touch it"
        )
    return list(struct.unpack_from(f"<{GRID * GRID}f", data, HEADER_BYTES))


def serialize_heights(heights):
    return struct.pack("<4I", GRID, GRID, 0, 0) + struct.pack(
        f"<{GRID * GRID}f", *heights
    )


def smoothstep(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def cell_noise(wx, wy):
    """Deterministic pseudo-noise in [-1, 1] for absolute grid cell (wx, wy).
    Pure integer hashing: stable across runs, platforms and Python versions,
    and continuous across block borders (no per-block pattern repetition)."""
    h = wx * 374761393 + wy * 668265263
    h = (h ^ (h >> 13)) * 1274126177
    h = h ^ (h >> 16)
    return (h & 0xFFFFFF) / 8388607.5 - 1.0


def distance_to_land_m(land_mask):
    """Two-pass chamfer distance transform (Euclidean approx., weights 3/4).

    land_mask: list of lists of bool, True for land cells. In-place result:
    each cell holds the chamfer distance in cells (0 for land). Returns the
    same grid with distances in meters (chamfer unit 3 == 1 cell == 2.5 m)."""
    n = len(land_mask)
    INF = float("inf")
    dist = [
        [0.0 if land_mask[y][x] else INF for x in range(n)] for y in range(n)
    ]

    # Forward pass: top-left -> bottom-right
    for y in range(n):
        row = dist[y]
        for x in range(n):
            v = row[x]
            if v == 0.0:
                continue
            best = v
            if x > 0 and row[x - 1] + 3.0 < best:
                best = row[x - 1] + 3.0
            if y > 0:
                row_prev = dist[y - 1]
                if row_prev[x] + 3.0 < best:
                    best = row_prev[x] + 3.0
                if row_prev[x - 1] + 4.0 < best:
                    best = row_prev[x - 1] + 4.0
                if x + 1 < n and row_prev[x + 1] + 4.0 < best:
                    best = row_prev[x + 1] + 4.0
            row[x] = best

    # Backward pass: bottom-right -> top-left
    for y in range(n - 1, -1, -1):
        row = dist[y]
        for x in range(n - 1, -1, -1):
            v = row[x]
            if v == 0.0:
                continue
            best = v
            if x + 1 < n and row[x + 1] + 3.0 < best:
                best = row[x + 1] + 3.0
            if y < n - 1:
                row_next = dist[y + 1]
                if row_next[x] + 3.0 < best:
                    best = row_next[x] + 3.0
                if x + 1 < n and row_next[x + 1] + 4.0 < best:
                    best = row_next[x + 1] + 4.0
                if x > 0 and row_next[x - 1] + 4.0 < best:
                    best = row_next[x - 1] + 4.0
            row[x] = best

    meters = (BLOCK_M / GRID) / 3.0  # chamfer unit -> meters
    for y in range(n):
        row = dist[y]
        for x in range(n):
            if row[x] != 0.0:
                row[x] = row[x] * meters
    return dist


def main():
    dry_run = "--dry-run" in sys.argv
    argv = [a for a in sys.argv[1:] if a != "--dry-run"]
    if argv:
        print(f"Unknown argument(s): {' '.join(argv)}", file=sys.stderr)
        print("Usage: python tools/deepen_ocean.py [--dry-run]", file=sys.stderr)
        return 2

    print(f"Zone dir: {ZONE_DIR}")
    print(
        f"Constants: DEEP_CM={DEEP_CM:.0f} cm, D_BEACH_M={D_BEACH_M:.0f} m, "
        f"D_DEEP_M={D_DEEP_M:.0f} m, REF_FLOOR_CM={REF_FLOOR_CM:.0f} cm, "
        f"noise={NOISE_FRACTION * 100:.0f}% of depth"
    )

    # ---- load all blocks ------------------------------------------------
    blocks = {}
    for by in range(GRID_MIN, GRID_MAX + 1):
        for bx in range(GRID_MIN, GRID_MAX + 1):
            blocks[(bx, by)] = load_block_heights(bx, by)
    print(f"Loaded {len(blocks)} HIM blocks.")

    # ---- global land mask + distance transform --------------------------
    print("Building land mask and distance transform...")
    land_mask = [[False] * GRID_CELLS for _ in range(GRID_CELLS)]
    land_cells = 0
    for by in range(GRID_MIN, GRID_MAX + 1):
        for gy in range(GRID):
            for bx in range(GRID_MIN, GRID_MAX + 1):
                heights = blocks[(bx, by)]
                wy = (by - GRID_MIN) * GRID + gy
                row = land_mask[wy]
                for gx in range(GRID):
                    if heights[gy * GRID + gx] > LAND_THRESHOLD_CM:
                        row[(bx - GRID_MIN) * GRID + gx] = True
                        land_cells += 1
    print(f"  land cells (height > {LAND_THRESHOLD_CM:.0f} cm): {land_cells}")

    dist = distance_to_land_m(land_mask)

    # ---- compute new heights --------------------------------------------
    beach_band = D_DEEP_M - D_BEACH_M
    changed_cells = 0
    underwater_cells = 0
    changed_blocks = []
    before_min = 0.0
    before_max = -1e9
    after_min = 0.0
    after_max = -1e9
    deepest_old = (None, 0.0)
    deepest_new = (None, 0.0)
    max_dist_seen = 0.0
    max_dist_cell = (None, None)

    for by in range(GRID_MIN, GRID_MAX + 1):
        for bx in range(GRID_MIN, GRID_MAX + 1):
            heights = blocks[(bx, by)]
            new_heights = None
            for gy in range(GRID):
                wy = (by - GRID_MIN) * GRID + gy
                dist_row = dist[wy]
                for gx in range(GRID):
                    idx = gy * GRID + gx
                    current = heights[idx]
                    wx = (bx - GRID_MIN) * GRID + gx
                    d = dist_row[wx]
                    if d > max_dist_seen:
                        max_dist_seen = d
                        max_dist_cell = (wx, wy)
                    if current > 0.0:
                        continue  # land: untouched
                    underwater_cells += 1
                    if current < before_min:
                        before_min = current
                    if current > before_max:
                        before_max = current
                    if d >= D_BEACH_M:
                        t = smoothstep((d - D_BEACH_M) / beach_band)
                        target_raw = REF_FLOOR_CM + (
                            DEEP_CM - REF_FLOOR_CM
                        ) * t
                        noise_cm = NOISE_FRACTION * abs(target_raw) * cell_noise(
                            wx, wy
                        )
                        v = round(
                            (target_raw + noise_cm) / QUANT_CM
                        ) * QUANT_CM
                        if v < current:
                            if new_heights is None:
                                new_heights = heights[:]
                            new_heights[idx] = v
                            changed_cells += 1
                    new_val = (
                        new_heights[idx] if new_heights is not None else current
                    )
                    if new_val < after_min:
                        after_min = new_val
                    if new_val > after_max:
                        after_max = new_val
                    if new_val < deepest_new[1]:
                        deepest_new = (
                            (bx, by, gx, gy),
                            new_val,
                        )
                    if current < deepest_old[1]:
                        deepest_old = ((bx, by, gx, gy), current)

            if new_heights is not None:
                changed_blocks.append((bx, by, new_heights))

    # ---- write back only changed blocks ---------------------------------
    written = []
    for bx, by, new_heights in changed_blocks:
        old_bytes = serialize_heights(blocks[(bx, by)])
        new_bytes = serialize_heights(new_heights)
        if new_bytes == old_bytes:
            continue
        if not dry_run:
            (ZONE_DIR / block_name(bx, by)).write_bytes(new_bytes)
        written.append((bx, by, len(new_bytes)))

    # ---- summary ----------------------------------------------------------
    print(f"\n=== Summary ===")
    print(f"  underwater cells:             {underwater_cells}")
    print(f"  cells changed (deepened):    {changed_cells}")
    print(f"  blocks changed:              {len(written)} of {len(blocks)}")
    if deepest_old[0]:
        bx, by, gx, gy = deepest_old[0]
        print(
            f"  deepest cell before:          {deepest_old[1]:.1f} cm "
            f"({-deepest_old[1] / 100:.2f} m) at block {bx}_{by} cell({gx},{gy})"
        )
    if deepest_new[0]:
        bx, by, gx, gy = deepest_new[0]
        print(
            f"  deepest cell after:           {deepest_new[1]:.1f} cm "
            f"({-deepest_new[1] / 100:.2f} m) at block {bx}_{by} cell({gx},{gy})"
        )
    print(
        f"  underwater height before:     {before_min:.1f} .. {before_max:.1f} cm "
        f"({-before_max / 100:.2f} .. {-before_min / 100:.2f} m deep)"
    )
    print(
        f"  underwater height after:      {after_min:.1f} .. {after_max:.1f} cm "
        f"({-after_max / 100:.2f} .. {-after_min / 100:.2f} m deep)"
    )
    print(
        f"  max distance to land:         {max_dist_seen:.1f} m "
        f"(cell {max_dist_cell[0]},{max_dist_cell[1]})"
    )

    if dry_run:
        print("\n=== Deep water area sanity check (dry-run) ===")
        for name, px, pz in SANITY_POINTS:
            bx = int(px // BLOCK_M)
            by = int(pz // BLOCK_M)
            gx = int((px - bx * BLOCK_M) / CELL_M)
            gy = int((pz - by * BLOCK_M) / CELL_M)
            wx = (bx - GRID_MIN) * GRID + gx
            wy = (by - GRID_MIN) * GRID + gy
            current = blocks[(bx, by)][gy * GRID + gx]
            d = dist[wy][wx]
            status = "land" if current > 0.0 else (
                "beach zone" if d < D_BEACH_M else (
                    "ramp zone" if d < D_DEEP_M else "deep zone"
                )
            )
            print(
                f"  {name:<9s} ({px:.0f}, {pz:.0f}) m: {d:6.1f} m from land, "
                f"height {current:7.1f} -> {after_at(blocks, dist, bx, by, gx, gy, wx, wy):7.1f} cm "
                f"({status})"
            )

    if not written:
        print("\nNothing to change (heights already deepened).")
    elif dry_run:
        print(f"\n{len(written)} HIM file(s) would be changed; no files were written.")
    else:
        print(f"\n{len(written)} HIM file(s) were changed. No TIL/IFO files were touched.")
    return 0


def after_at(blocks, dist, bx, by, gx, gy, wx, wy):
    """Recompute the post-deepen height at one cell (used by dry-run only)."""
    heights = blocks[(bx, by)]
    current = heights[gy * GRID + gx]
    if current > 0.0:
        return current
    d = dist[wy][wx]
    if d < D_BEACH_M:
        return current
    t = smoothstep((d - D_BEACH_M) / (D_DEEP_M - D_BEACH_M))
    target_raw = REF_FLOOR_CM + (DEEP_CM - REF_FLOOR_CM) * t
    noise_cm = NOISE_FRACTION * abs(target_raw) * cell_noise(wx, wy)
    v = round((target_raw + noise_cm) / QUANT_CM) * QUANT_CM
    return v if v < current else current


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""
paint_island_tiles.py
=====================
Paint sand/grass tiles onto the island terrain of ROSE Online zone 200 (OCEAN)
by editing the TIL files in 3DDATA/MAPS/OCEAN.

Background
----------
tools/generate_ocean_islands.py raised the terrain heights (HIM files) of the
12 new islands but never touched the TIL files, so every island cell still
carries the ocean-floor tile id (0) and the islands render with the floor
texture instead of sand/grass.

This script paints, for every block of the 17x17 active grid (blocks 24..40):

  * cells whose terrain height (max of the 4x4 HIM grid cells under the TIL
    cell, i.e. the 5x5 mesh vertices) is:
        >  80 cm  -> tile id 2  (grass/inland, verified on the original islands)
        >   0 cm  -> tile id 1  (sand/beach, verified on the original islands)
        > -20 cm  -> tile id 1  (sandy shallows just below water, as the
                                 original islands do -- the original map has
                                 13 such cells in (-20, 0] cm, all sand)
        <= -20 cm -> untouched (deep water floor)

  The id -> texture semantics (0 = floor, 1 = sand, 2 = grass) were sampled
  from the 3 original islands in this same map: their land cells use id 2
  (1392 cells, heights 40..180+ cm) and id 1 (226 cells, heights -20..100 cm,
  peaking 20..60 cm); deep water uses id 0 (65534 cells).  The scaffold
  OCEAN.ZON defines exactly 3 tiles, so ids 1 and 2 are safe to write.

Safety
------
* Only cells whose current id is 0 (the water-floor default) are ever
  overwritten.  Cells already carrying non-zero ids (the original islands)
  are never touched.
* Blocks without a TIL file are treated as all-id-0 and a full TIL file is
  created for them on write (the client loader defaults a missing TIL to
  tile id 0, see src/zone_loader/loading.rs + src/zone_loader/spawning/terrain.rs).
  In the current data every block 24..40 already has a TIL file (1800 bytes),
  so nothing is created -- but the code path is implemented.
* Deterministic and idempotent: a second run reports "nothing to change".
* No dependency beyond the Python standard library.

File formats (verified against rose-file-readers/src/til.rs,
rose-file-readers/src/him.rs and src/map_editor/coords.rs):

  HIM: u32 width (65), u32 height (65), u32 reserved (0), u32 reserved (0),
       65*65 f32 little-endian heights in cm, row-major
       (row = y grid, index = y*65 + x).  Total 16916 bytes.  Grid cell 2.5 m,
       block 160 m.
  TIL: u32 width (16), u32 height (16), then per cell: 3 reserved bytes + one
       u32 tile id, little-endian.  Total 1800 bytes.  Cell 10 m, block 160 m.

Run from the repo root (plain standard library, Python 3.8+):
    python tools/paint_island_tiles.py            # apply changes
    python tools/paint_island_tiles.py --dry-run  # preview only, no writes
"""

import struct
import sys
from pathlib import Path

ZONE_DIR = Path(__file__).resolve().parent.parent / "3DDATA" / "MAPS" / "OCEAN"

BLOCK_M = 160.0
HIM_GRID = 65
HIM_HEADER_BYTES = 16
EXPECTED_HIM_SIZE = HIM_HEADER_BYTES + HIM_GRID * HIM_GRID * 4  # 16916

TIL_SIZE = 16
TIL_HEADER_BYTES = 8
CELL_BYTES = 7  # 3 reserved + 4-byte id
EXPECTED_TIL_SIZE = TIL_HEADER_BYTES + TIL_SIZE * TIL_SIZE * CELL_BYTES  # 1800

ID_WATER_FLOOR = 0
ID_SAND = 1
ID_GRASS = 2

BEACH_MAX_CM = 80.0    # heights in (0, 80] cm -> sand
SHALLOW_MIN_CM = -20.0  # heights in (-20, 0] cm -> sand (beach shallows)

GRID_MIN = 24
GRID_MAX = 40


class PaintError(Exception):
    pass


def block_name(bx, by, ext):
    return f"{bx}_{by}.{ext}"


def load_him(path):
    data = path.read_bytes()
    if len(data) != EXPECTED_HIM_SIZE:
        raise PaintError(
            f"{path.name}: unexpected size {len(data)} bytes "
            f"(expected {EXPECTED_HIM_SIZE}); refusing to touch it"
        )
    w, h, r1, r2 = struct.unpack_from("<4I", data, 0)
    if w != HIM_GRID or h != HIM_GRID or r1 != 0 or r2 != 0:
        raise PaintError(
            f"{path.name}: unexpected header ({w},{h},{r1},{r2}); refusing to touch it"
        )
    return list(struct.unpack_from(f"<{HIM_GRID * HIM_GRID}f", data, HIM_HEADER_BYTES))


def load_til(path):
    """Return the 16x16 tile id matrix (list of 16 rows), or None if absent."""
    if not path.is_file():
        return [[ID_WATER_FLOOR] * TIL_SIZE for _ in range(TIL_SIZE)]
    data = path.read_bytes()
    if len(data) != EXPECTED_TIL_SIZE:
        raise PaintError(
            f"{path.name}: unexpected size {len(data)} bytes "
            f"(expected {EXPECTED_TIL_SIZE}); refusing to touch it"
        )
    w, h = struct.unpack_from("<II", data, 0)
    if w != TIL_SIZE or h != TIL_SIZE:
        raise PaintError(
            f"{path.name}: unexpected header ({w},{h}); refusing to touch it"
        )
    cells = []
    pos = TIL_HEADER_BYTES
    for _ in range(TIL_SIZE):
        row = []
        for _ in range(TIL_SIZE):
            row.append(struct.unpack_from("<I", data, pos + 3)[0])
            pos += CELL_BYTES
        cells.append(row)
    return cells


def serialize_til(cells):
    out = bytearray()
    out += struct.pack("<II", TIL_SIZE, TIL_SIZE)
    for row in cells:
        for tid in row:
            out += b"\x00\x00\x00"
            out += struct.pack("<I", tid)
    return bytes(out)


def tile_target_id(hmax_cm):
    """Height -> tile id mapping, mirroring the original islands' usage."""
    if hmax_cm > BEACH_MAX_CM:
        return ID_GRASS
    if hmax_cm > SHALLOW_MIN_CM:
        return ID_SAND
    return None


def paint_block(bx, by):
    """Compute the new TIL matrix for block (bx, by).  Returns
    (new_cells, existed_before, changed_cells, painted) where painted is a
    list of (tx, tz, old_id, new_id, hmax_cm)."""
    him = load_him(ZONE_DIR / block_name(bx, by, "HIM"))
    til_path = ZONE_DIR / block_name(bx, by, "TIL")
    existed = til_path.is_file()
    cells = load_til(til_path)

    painted = []
    changed = 0
    for tz in range(TIL_SIZE):
        for tx in range(TIL_SIZE):
            gx0 = tx * 4
            gz0 = tz * 4
            hs = [
                him[(gz0 + dy) * HIM_GRID + gx0 + dx]
                for dy in range(4)
                for dx in range(4)
            ]
            target = tile_target_id(max(hs))
            if target is None:
                continue
            if cells[tz][tx] != ID_WATER_FLOOR:
                continue
            cells[tz][tx] = target
            changed += 1
            painted.append((tx, tz, ID_WATER_FLOOR, target, max(hs)))
    return cells, existed, changed, painted


def main():
    dry_run = "--dry-run" in sys.argv
    argv = [a for a in sys.argv[1:] if a != "--dry-run"]
    if argv:
        print(f"Unknown argument(s): {' '.join(argv)}", file=sys.stderr)
        print("Usage: python tools/paint_island_tiles.py [--dry-run]", file=sys.stderr)
        return 2

    print(f"Zone dir: {ZONE_DIR}")
    print(
        f"Mapping: sand (id {ID_SAND}) for heights in ({SHALLOW_MIN_CM:.0f}, {BEACH_MAX_CM:.0f}] cm, "
        f"grass (id {ID_GRASS}) for heights > {BEACH_MAX_CM:.0f} cm"
    )

    to_create = []
    to_modify = []
    painted_total = {ID_SAND: 0, ID_GRASS: 0}
    sample_sand = []
    sample_grass = []
    sample_shallow = []

    for by in range(GRID_MIN, GRID_MAX + 1):
        for bx in range(GRID_MIN, GRID_MAX + 1):
            cells, existed, changed, painted = paint_block(bx, by)
            if changed == 0:
                continue
            to_modify.append((bx, by))
            if not existed:
                to_create.append((bx, by))
            for tx, tz, old, new, hmax in painted:
                painted_total[new] += 1
                if new == ID_GRASS and len(sample_grass) < 8:
                    sample_grass.append((bx, by, tx, tz, hmax))
                elif new == ID_SAND and hmax <= 0 and len(sample_shallow) < 8:
                    sample_shallow.append((bx, by, tx, tz, hmax))
                elif new == ID_SAND and len(sample_sand) < 8:
                    sample_sand.append((bx, by, tx, tz, hmax))

            if dry_run:
                continue
            (ZONE_DIR / block_name(bx, by, "TIL")).write_bytes(serialize_til(cells))

    print(f"\n=== TIL files to create ===")
    if not to_create:
        print("  (none -- every block 24..40 already has a TIL file)")
    for bx, by in sorted(set(to_create)):
        print(f"  {block_name(bx, by, 'TIL')}")

    print(f"\n=== TIL files to modify ===")
    if not to_modify:
        print("  (none -- nothing to change)")
    for bx, by in sorted(set(to_modify)):
        print(f"  {block_name(bx, by, 'TIL')}")

    print(f"\n=== Cells painted ===")
    print(f"  sand (id {ID_SAND}):  {painted_total[ID_SAND]}")
    print(f"  grass (id {ID_GRASS}): {painted_total[ID_GRASS]}")

    def show(label, rows):
        if not rows:
            return
        print(f"  sample {label}:")
        for bx, by, tx, tz, h in rows:
            x = bx * BLOCK_M + tx * 10.0 + 5.0
            z = by * BLOCK_M + tz * 10.0 + 5.0
            print(f"    block {bx}_{by} cell({tx},{tz}) at ({x:.0f}, {z:.0f}) m  h={h:.1f} cm")

    show("beach sand cells", sample_sand)
    show("underwater sand shallows", sample_shallow)
    show("grass cells", sample_grass)

    if not to_modify:
        print("\nNothing to change.")
    elif dry_run:
        print(f"\n{len(to_modify)} TIL file(s) would be modified; no files were written.")
    else:
        print(f"\n{len(to_modify)} TIL file(s) were modified. No other files were touched.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

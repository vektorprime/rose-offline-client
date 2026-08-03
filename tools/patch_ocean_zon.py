#!/usr/bin/env python3
"""
patch_ocean_zon.py
==================
Remap the tile-definition entries of the zone 200 (OCEAN) ZON file so that the
island tiles painted by tools/paint_island_tiles.py (TIL tile id 1 = sand,
id 2 = grass) actually render as sand/grass instead of the flat gray texture.

Background
----------
3DDATA/MAPS/OCEAN/OCEAN.ZON is a hand-written scaffold whose Tiles block maps
every TIL id (0, 1, 2) to texture index 0 (`3DDATA/TERRAIN/TILES/JUNON/JD/
S001_01.DDS`), so the painted ids change nothing visually.  This script:

  * keeps tile id 0 (deep water floor) on the current texture,
  * appends two real textures to the ZON's texture list:
      - id 1 (sand)  -> S001_02.DDS  (golden sand, avg RGB 156,130,65)
      - id 2 (grass) -> T011_01.DDS  (warm brown land, avg RGB 168,129,81)
    (color values measured from the DXT3 pixel data of the actual files in
    target/debug/3Ddata/TERRAIN/TILES/JUNON/JD/; the current S001_01.DDS
    measures avg RGB 168,166,134 -- the flat gray),
  * repoints tiles 1 and 2 of the Tiles block at the new indices the way the
    renderer resolves them (see below),
  * leaves every other byte of the file untouched (see preservation below).

ZON file format (verified against rose-file-readers/src/zon.rs and the actual
OCEAN.ZON binary):
  * u32 block_count, then per block: u32 block_type, u32 block_offset.
  * Blocks (zon.rs:67-131), each at its own offset:
      type 0 ZoneInfo:     12 reserved bytes, u32 grid_per_patch, f32
                           grid_size, 8 reserved bytes (zon.rs:75-81).
      type 1 EventPos:     u32 count, then per event: Vec3<f32> + u8-length
                           string (zon.rs:83-92).
      type 2 Textures:     u32 count, then per texture: u8 length + bytes,
                           no terminator (zon.rs:94-101,
                           reader.rs:280-283).
      type 3 Tiles:        u32 count, then per tile 7 x u32 LE:
                           layer1, layer2, offset1, offset2, blend,
                           rotation, reserved (zon.rs:103-125).
      type 4 Economy:      ignored (zon.rs:127).
  * The renderer resolves the texture for a TIL id as
    tile_textures[layer1 + offset1] and tile_textures[layer2 + offset2]
    (src/zone_loader/spawning/terrain.rs:52-53, 91-102), where the TIL id is
    the index into zon.tiles (terrain.rs:43-51).  The texture strings are
    loaded by the client in list order (src/zone_loader/spawning.rs:81-91;
    loading stops at the sentinel "end").
  * A real zone (3DDATA/MAPS/JUNON/JD01/JD01.ZON) uses layer1=N, offset1=0
    style pointers, so this script follows the same convention.

Preservation
------------
The texture block grows by two entries, so every block that follows it shifts;
the file is therefore re-serialized in full: headers are re-emitted with
recomputed offsets and non-texture/tile blocks (ZoneInfo, EventPos, unknown
types) are copied byte-for-byte as raw payloads.  Strings are re-encoded with
their exact original bytes, and all untouched tiles keep their exact u32
fields, so the only differing bytes in the output are: the texture count, the
two appended texture entries, the tiles-block header offset, and the
layer1/layer2 fields of tiles 1 and 2.  Idempotent: a second run produces
byte-identical output and reports "already up to date".

Targets
-------
Two OCEAN.ZON copies exist and are currently byte-identical:
  * 3DDATA/MAPS/OCEAN/OCEAN.ZON  (repo staging copy edited by the other
                                  tools: generate_ocean_islands.py,
                                  paint_island_tiles.py)
  * target/debug/3Ddata/MAPS/OCEAN/OCEAN.ZON  (runtime copy the client loads;
                                  the only tree that contains the terrain
                                  DDS files)
Both are patched independently (each is idempotent), keeping them in sync.

Run from the repo root (plain standard library, Python 3.8+):
    python tools/patch_ocean_zon.py            # apply changes
    python tools/patch_ocean_zon.py --dry-run  # preview only, no writes
"""

import os
import struct
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

BLOCK_TEXTURES = 2
BLOCK_TILES = 3

TILE_FLOOR = 0
TILE_SAND = 1
TILE_GRASS = 2

# Paths/case match the existing entry in OCEAN.ZON
# ("3DDATA/TERRAIN/TILES/JUNON/JD/S001_01.DDS", 41 chars).  Both files exist
# in target/debug/3Ddata/TERRAIN/TILES/JUNON/JD/ (82048 bytes each).
TEXTURE_SAND = "3DDATA/TERRAIN/TILES/JUNON/JD/S001_02.DDS"
TEXTURE_GRASS = "3DDATA/TERRAIN/TILES/JUNON/JD/T011_01.DDS"

ZON_CANDIDATES = (
    REPO_ROOT / "3DDATA" / "MAPS" / "OCEAN" / "OCEAN.ZON",
    REPO_ROOT / "target" / "debug" / "3Ddata" / "MAPS" / "OCEAN" / "OCEAN.ZON",
)


class PatchError(Exception):
    pass


# ---------------------------------------------------------------- parsing

def read_u32(data, pos):
    if pos + 4 > len(data):
        raise PatchError(f"unexpected end of file at offset {pos} (need 4 bytes)")
    return struct.unpack_from("<I", data, pos)[0], pos + 4


def read_string(data, pos):
    if pos + 1 > len(data):
        raise PatchError(f"unexpected end of file at offset {pos} (need 1 length byte)")
    ln = data[pos]
    end = pos + 1 + ln
    if end > len(data):
        raise PatchError(
            f"string length {ln} at offset {pos} overruns end of file ({len(data)} bytes)"
        )
    return data[pos + 1 : end], end


def parse_textures(data, pos):
    count, pos = read_u32(data, pos)
    textures = []
    for _ in range(count):
        raw, pos = read_string(data, pos)
        textures.append(raw)
    return textures, pos


def parse_tiles(data, pos):
    count, pos = read_u32(data, pos)
    tiles = []
    for _ in range(count):
        if pos + 28 > len(data):
            raise PatchError(
                f"tile {len(tiles)} at offset {pos} overruns end of file ({len(data)} bytes)"
            )
        fields = struct.unpack_from("<7I", data, pos)
        pos += 28
        tiles.append(fields)
    return tiles, pos


def parse_zon(data):
    """Return a dict describing the ZON file.

    blocks: list of dicts {type, offset, payload} where payload is the raw
    block body (everything after the block's header), plus 'textures' or
    'tiles' parsed views where applicable.
    """
    block_count, pos = read_u32(data, 0)
    blocks = []
    for i in range(block_count):
        btype, after_type = read_u32(data, pos)
        boff, after_offset = read_u32(data, after_type)
        pos = after_offset
        if after_offset > len(data):
            raise PatchError(f"block header {i} overruns end of file")
        if btype == BLOCK_TEXTURES:
            textures, body_end = parse_textures(data, boff)
            blocks.append(
                {"type": btype, "offset": boff, "payload": data[boff:body_end], "textures": textures}
            )
        elif btype == BLOCK_TILES:
            tiles, body_end = parse_tiles(data, boff)
            blocks.append(
                {"type": btype, "offset": boff, "payload": data[boff:body_end], "tiles": tiles}
            )
        else:
            blocks.append({"type": btype, "offset": boff, "payload": None})
    return finish_block_payloads(data, blocks)


def finish_block_payloads(data, blocks):
    """Fill raw payloads for non-texture/tile blocks using the next block's
    offset (or end of file) as the body boundary."""
    ordered = sorted(enumerate(blocks), key=lambda e: e[1]["offset"])
    for idx, (_, blk) in enumerate(ordered):
        if blk["payload"] is None:
            if idx + 1 < len(ordered):
                end = ordered[idx + 1][1]["offset"]
            else:
                end = len(data)
            blk["payload"] = data[blk["offset"]:end]
    return {"block_count": block_count_of(blocks), "blocks": blocks}


def block_count_of(blocks):
    return len(blocks)


# ---------------------------------------------------------------- encoding

def encode_string(raw):
    return bytes([len(raw)]) + raw


def encode_textures(textures):
    out = bytearray(struct.pack("<I", len(textures)))
    for raw in textures:
        out += encode_string(raw)
    return bytes(out)


def encode_tiles(tiles):
    out = bytearray(struct.pack("<I", len(tiles)))
    for t in tiles:
        out += struct.pack("<7I", *t)
    return bytes(out)


def serialize_zon(parsed):
    """Re-serialize a parsed ZON, recomputing every block offset.  Payloads
    are emitted in the original physical order (by original offset)."""
    header_size = 4 + 8 * parsed["block_count"]
    body_offset = header_size
    encoded = []
    for blk in sorted(parsed["blocks"], key=lambda b: b["offset"]):
        if blk["type"] == BLOCK_TEXTURES:
            body = encode_textures(blk["textures"])
        elif blk["type"] == BLOCK_TILES:
            body = encode_tiles(blk["tiles"])
        else:
            body = blk["payload"]
        encoded.append((blk["type"], body_offset, body))
        body_offset += len(body)

    out = bytearray(struct.pack("<I", parsed["block_count"]))
    for btype, boff, _ in encoded:
        out += struct.pack("<II", btype, boff)
    for _, _, body in encoded:
        out += body
    return bytes(out)


# ---------------------------------------------------------------- patching

def texture_index(textures, name):
    for i, raw in enumerate(textures):
        if raw.decode("ascii", "replace").lower() == name.lower():
            return i
    return None


def build_patched(data):
    """Return (before_info, after_bytes, after_info).  Raises PatchError if
    the file does not match the expected scaffold shape."""
    parsed = parse_zon(data)
    tex_block = next((b for b in parsed["blocks"] if b["type"] == BLOCK_TEXTURES), None)
    til_block = next((b for b in parsed["blocks"] if b["type"] == BLOCK_TILES), None)
    if tex_block is None:
        raise PatchError("no Textures block found in ZON; refusing to patch")
    if til_block is None:
        raise PatchError("no Tiles block found in ZON; refusing to patch")
    if len(til_block["tiles"]) <= TILE_GRASS:
        raise PatchError(
            f"Tiles block defines only {len(til_block['tiles'])} tile(s); "
            f"tile ids {TILE_SAND} and {TILE_GRASS} must exist (the scaffold defines 3)"
        )

    before = {
        "textures": [t.decode("ascii", "replace") for t in tex_block["textures"]],
        "tiles": {
            i: {
                "idx": til_block["tiles"][i][0] + til_block["tiles"][i][2],
                "fields": til_block["tiles"][i],
            }
            for i in range(0, len(til_block["tiles"]))
        },
    }

    textures = list(tex_block["textures"])
    sand_idx = texture_index(textures, TEXTURE_SAND)
    if sand_idx is None:
        textures.append(TEXTURE_SAND.encode("ascii"))
        sand_idx = len(textures) - 1
    grass_idx = texture_index(textures, TEXTURE_GRASS)
    if grass_idx is None:
        textures.append(TEXTURE_GRASS.encode("ascii"))
        grass_idx = len(textures) - 1
    tex_block["textures"] = textures

    tiles = list(til_block["tiles"])
    for tid, idx in ((TILE_SAND, sand_idx), (TILE_GRASS, grass_idx)):
        l1, l2, o1, o2, blend, rotation, reserved = tiles[tid]
        tiles[tid] = (idx, idx, 0, 0, blend, rotation, reserved)
    til_block["tiles"] = tiles

    after_bytes = serialize_zon(parsed)

    # Round-trip: the rewritten file must re-parse to the same structure.
    re_parsed = parse_zon(after_bytes)
    re_tex = next(b for b in re_parsed["blocks"] if b["type"] == BLOCK_TEXTURES)
    re_til = next(b for b in re_parsed["blocks"] if b["type"] == BLOCK_TILES)
    if re_tex["textures"] != textures or re_til["tiles"] != tiles:
        raise PatchError("internal error: round-trip validation failed")

    after = {
        "textures": [t.decode("ascii", "replace") for t in textures],
        "tiles": {
            i: {
                "idx": tiles[i][0] + tiles[i][2],
                "fields": tiles[i],
            }
            for i in range(0, len(tiles))
        },
    }
    return before, after_bytes, after


# ---------------------------------------------------------------- reporting

def summarize(path, before, after, changed, dds_missing):
    print(f"\n=== {path}")
    print(f"  tile textures:")
    for i, name in enumerate(before["textures"]):
        print(f"    before [{i}] {name}")
    for i, name in enumerate(after["textures"]):
        print(f"    after  [{i}] {name}")
    print(f"  tile defs (TIL id -> texture = layer1 + offset1):")
    ids = sorted(set(before["tiles"]) | set(after["tiles"]))
    for i in ids:
        b, a = before["tiles"][i], after["tiles"][i]
        arrow = "->" if (b["idx"], b["fields"]) != (a["idx"], a["fields"]) else "  "
        print(
            f"    id {i}: before {b['idx']} (layer1={b['fields'][0]}, offset1={b['fields'][2]}) "
            f"{arrow} after {a['idx']} (layer1={a['fields'][0]}, offset1={a['fields'][2]})"
        )
    if dds_missing:
        print(f"  WARNING: one or more target DDS files are missing in this data tree:")
        for p in dds_missing:
            print(f"    MISSING: {p}")
    if changed:
        print(f"  would change ({len(before['textures'])} -> {len(after['textures'])} textures)")
    else:
        print(f"  already up to date (no change)")


# ---------------------------------------------------------------- main

def process(zon_path, dry_run):
    if not zon_path.is_file():
        print(f"\n=== {zon_path}\n  (not present -- skipping)")
        return True

    data = zon_path.read_bytes()
    try:
        before, after_bytes, after = build_patched(data)
    except PatchError as e:
        print(f"\n=== {zon_path}\n  ERROR: {e}")
        return False

    tree_root = zon_path.parents[3]  # the tree that contains the 3DDATA folder
    dds_missing = []
    for name in (TEXTURE_SAND, TEXTURE_GRASS):
        if not (tree_root / name).is_file():
            dds_missing.append(str(tree_root / name))

    changed = after_bytes != data
    summarize(zon_path, before, after, changed, dds_missing)

    if not changed:
        return True
    if dry_run:
        print(f"  {zon_path.name}: {len(data)} -> {len(after_bytes)} bytes (dry run, no write)")
        return True

    tmp = zon_path.with_name(zon_path.name + ".tmp")
    tmp.write_bytes(after_bytes)
    os.replace(tmp, zon_path)
    print(f"  patched: {len(data)} -> {len(after_bytes)} bytes")
    return True


def main():
    dry_run = "--dry-run" in sys.argv
    argv = [a for a in sys.argv[1:] if a != "--dry-run"]
    if argv:
        print(f"Unknown argument(s): {' '.join(argv)}", file=sys.stderr)
        print("Usage: python tools/patch_ocean_zon.py [--dry-run]", file=sys.stderr)
        return 2

    print(
        "Tile texture remap for OCEAN.ZON:\n"
        f"  TIL id {TILE_SAND}  -> {TEXTURE_SAND}  (golden sand)\n"
        f"  TIL id {TILE_GRASS} -> {TEXTURE_GRASS}  (warm brown land)\n"
        f"  TIL id {TILE_FLOOR} -> unchanged (deep water floor)"
    )

    ok = True
    for zon_path in ZON_CANDIDATES:
        ok = process(zon_path, dry_run) and ok

    if dry_run:
        print("\nDry run complete -- no files were written.")
    else:
        print("\nDone. If you changed the runtime copy under target/debug/3Ddata,")
        print("restart the client so the zone is reloaded.")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

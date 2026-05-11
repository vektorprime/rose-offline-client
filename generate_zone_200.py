#!/usr/bin/env python3
"""
Generate Zone 200 (Sailing Zone) data files for Rose Offline Client.
Creates: ZON, HIM, TIL, IFO binary files.
"""

import struct
import math
import os
import json

ZONE_ID = 200
ZONE_NAME = "SailingZone"
ZONE_DIR = f"/tmp/rose-offline-client/data/zones/{ZONE_ID}"

# Zone dimensions (same as existing zones)
ZONE_SIZE = 10400  # meters (10.4km x 10.4km)
BLOCK_SIZE = 160  # meters
GRID_SIZE = 64  # 64x64 blocks

# Water level in cm (same as existing zones)
WATER_LEVEL_CM = 0.0
SEA_FLOOR_CM = -500.0  # 5m below water

# Island definitions: (center_x, center_z, radius_m, peak_height_m)
ISLANDS = [
    # Main hub island (center)
    (5200, -5200, 300, 15),
    # Small islands scattered around
    (2000, -2000, 150, 10),
    (8000, -2000, 120, 8),
    (2000, -8000, 180, 12),
    (8000, -8000, 100, 7),
    (5200, -2000, 200, 11),
    (5200, -8000, 160, 9),
    (3000, -5200, 140, 8),
    (7400, -5200, 130, 10),
    (1000, -5200, 100, 6),
    (9400, -5200, 110, 7),
    (5200, -1000, 90, 5),
    (5200, -9400, 80, 6),
    # Tiny islets
    (3500, -3500, 50, 3),
    (6900, -3500, 40, 2),
    (3500, -6900, 45, 3),
    (6900, -6900, 35, 2),
]

# Dock positions: (island_index, offset_x, offset_z, rotation_deg)
DOCKS = [
    (0, 0, 100, 0),     # Main hub - south dock
    (0, 100, 0, 90),    # Main hub - east dock
    (1, 0, 50, 0),      # Island 1 - south
    (2, 0, -50, 180),   # Island 2 - north
    (3, 50, 0, 90),     # Island 3 - east
    (4, -50, 0, 270),   # Island 4 - west
    (5, 0, 70, 0),      # Island 5 - south
    (6, 0, -70, 180),   # Island 6 - north
    (7, 70, 0, 90),     # Island 7 - east
    (8, -70, 0, 270),   # Island 8 - west
]

def ensure_dirs():
    os.makedirs(ZONE_DIR, exist_ok=True)
    os.makedirs(f"{ZONE_DIR}/him", exist_ok=True)
    os.makedirs(f"{ZONE_DIR}/til", exist_ok=True)
    os.makedirs(f"{ZONE_DIR}/ifo", exist_ok=True)

def generate_zon():
    """Generate ZON zone info file."""
    # ZON format: zone_id(u16) + zone_name_len(u8) + zone_name(bytes)
    # + zone_type(u32) + unknown padding
    zone_name_bytes = ZONE_NAME.encode('utf-8')
    
    data = struct.pack('<H', ZONE_ID)  # zone_id
    data += struct.pack('<B', len(zone_name_bytes))  # name length
    data += zone_name_bytes  # name
    data += struct.pack('<I', 1)  # zone_type (1 = outdoor)
    data += struct.pack('<I', 0)  # padding
    
    with open(f"{ZONE_DIR}/zone.zon", 'wb') as f:
        f.write(data)
    print(f"Generated zone.zon ({len(data)} bytes)")

def height_at(x, z):
    """Calculate terrain height at (x, z) in meters."""
    height = SEA_FLOOR_CM  # Start at sea floor
    
    for (cx, cz, radius, peak) in ISLANDS:
        dx = x - cx
        dz = z - cz
        dist = math.sqrt(dx*dx + dz*dz)
        
        if dist < radius:
            # Smooth height falloff using cosine
            falloff = 0.5 * (1 + math.cos(math.pi * dist / radius))
            island_height = peak * falloff * falloff  # Squared for smoother peaks
            height = max(height, island_height * 100)  # Convert to cm
    
    return height

def generate_him():
    """Generate HIM heightmap file."""
    # HIM format: width(u32) + height(u32) + padding(2x u32) + heightmap(f32 per cell)
    # Grid size: 257x257 (standard for Rose Online zones)
    grid_size = 257
    cell_size = ZONE_SIZE / (grid_size - 1)  # meters per cell
    
    heights = []
    for gz in range(grid_size):
        for gx in range(grid_size):
            x = gx * cell_size
            z = -gz * cell_size  # Z is negative in Rose Online
            h = height_at(x, z)
            heights.append(h)
    
    data = struct.pack('<I', grid_size)  # width
    data += struct.pack('<I', grid_size)  # height
    data += struct.pack('<I', 0)  # padding
    data += struct.pack('<I', 0)  # padding
    
    for h in heights:
        data += struct.pack('<f', h)  # height in cm
    
    with open(f"{ZONE_DIR}/him/0.him", 'wb') as f:
        f.write(data)
    print(f"Generated 0.him ({len(data)} bytes, {grid_size}x{grid_size})")

def generate_til():
    """Generate TIL tilemap file."""
    # TIL format: width(u32) + height(u32) + tiles(3 bytes padding + u32 tile ID per cell)
    grid_size = 257
    cell_size = ZONE_SIZE / (grid_size - 1)
    
    tiles = []
    for gz in range(grid_size):
        for gx in range(grid_size):
            x = gx * cell_size
            z = -gz * cell_size
            h = height_at(x, z)
            
            # Tile ID: 0 = water, 1 = sand, 2 = grass, 3 = rock
            if h < -100:  # Deep water
                tile_id = 0
            elif h < 0:   # Shallow water
                tile_id = 0
            elif h < 50:  # Beach/sand
                tile_id = 1
            elif h < 500: # Grass
                tile_id = 2
            else:         # Rock/mountain
                tile_id = 3
            
            tiles.append(tile_id)
    
    data = struct.pack('<I', grid_size)  # width
    data += struct.pack('<I', grid_size)  # height
    
    for tile_id in tiles:
        data += struct.pack('<3s', b'\x00\x00\x00')  # 3 bytes padding
        data += struct.pack('<I', tile_id)  # tile ID
    
    with open(f"{ZONE_DIR}/til/0.til", 'wb') as f:
        f.write(data)
    print(f"Generated 0.til ({len(data)} bytes, {grid_size}x{grid_size})")

def deg_to_quat(deg):
    """Convert degrees to quaternion [x, y, z, w]."""
    rad = math.radians(deg)
    return [0, math.sin(rad/2), 0, math.cos(rad/2)]

def write_u8_string(data, s):
    """Write u8 length-prefixed string."""
    encoded = s.encode('utf-8')[:255]
    data += struct.pack('<B', len(encoded))
    data += encoded
    return data

def write_object(data, obj_name, warp_id, event_id, obj_type, obj_id, 
                 minimap_x, minimap_y, rotation, position, scale):
    """Write a standard IFO object."""
    data = write_u8_string(data, obj_name)
    data += struct.pack('<H', warp_id)
    data += struct.pack('<H', event_id)
    data += struct.pack('<I', obj_type)
    data += struct.pack('<I', obj_id)
    data += struct.pack('<I', minimap_x)
    data += struct.pack('<I', minimap_y)
    for v in rotation:
        data += struct.pack('<f', v)
    for v in position:
        data += struct.pack('<f', v)
    for v in scale:
        data += struct.pack('<f', v)
    return data

def generate_ifo():
    """Generate IFO file with docks, NPCs, monster spawns, and water planes."""
    
    # Block types
    DECO_OBJECT = 1
    NPC = 2
    CNST_OBJECT = 3
    SOUND_OBJECT = 4
    EFFECT_OBJECT = 5
    ANIMATED_OBJECT = 6
    MONSTER_SPAWN = 8
    WATER_PLANES = 9
    WARP = 10
    COLLISION_OBJECT = 11
    EVENT_OBJECT = 12
    
    # Collect objects by block type
    block_data = {}
    
    # === Water planes (full zone coverage) ===
    water_data = struct.pack('<f', 0.0)  # water_size
    plane_count = 4
    water_data += struct.pack('<I', plane_count)
    
    # 4 water planes covering the zone corners
    for start_x, start_z, end_x, end_z in [
        (0, 0, ZONE_SIZE, -ZONE_SIZE),
        (0, -ZONE_SIZE, ZONE_SIZE, 0),
        (-ZONE_SIZE, 0, 0, -ZONE_SIZE),
        (-ZONE_SIZE, -ZONE_SIZE, 0, 0),
    ]:
        # Start position
        for v in [start_x * 100, start_z * 100, WATER_LEVEL_CM]:
            water_data += struct.pack('<f', v)
        # End position
        for v in [end_x * 100, end_z * 100, WATER_LEVEL_CM]:
            water_data += struct.pack('<f', v)
    
    block_data[WATER_PLANES] = water_data
    
    # === Docks (as construction objects) ===
    cnst_data = struct.pack('<I', len(DOCKS))  # count
    for idx, (island_idx, off_x, off_z, rot_deg) in enumerate(DOCKS):
        cx, cz, radius, _ = ISLANDS[island_idx]
        x = cx + off_x
        z = cz + off_z
        
        # Position in IFO coords (cm)
        position = [x * 100, z * 100, 50]  # 0.5m above water
        
        rotation = deg_to_quat(rot_deg)
        
        cnst_data = write_object(cnst_data,
            f"dock_{idx}", 0, 0, 0, idx + 100,
            0, 0, rotation, position, [1, 1, 1])
    
    block_data[CNST_OBJECT] = cnst_data
    
    # === Warp points (docks warp to main hub) ===
    warp_data = struct.pack('<I', len(DOCKS))  # count
    for idx, (island_idx, off_x, off_z, rot_deg) in enumerate(DOCKS):
        cx, cz, radius, _ = ISLANDS[island_idx]
        x = cx + off_x
        z = cz + off_z
        
        position = [x * 100, z * 100, 100]
        rotation = deg_to_quat(rot_deg)
        
        warp_data = write_object(warp_data,
            f"warp_{idx}", idx + 1, 0, 0, idx + 200,
            0, 0, rotation, position, [1, 1, 1])
    
    block_data[WARP] = warp_data
    
    # === Monster spawns (pirate ships, sea creatures) ===
    monster_data = struct.pack('<I', 6)  # 6 spawn points
    
    # Pirate ship spawn points
    pirate_positions = [
        (3000, -3000, "pirate_ship_1"),
        (7000, -7000, "pirate_ship_2"),
        (5200, -3000, "pirate_ship_3"),
    ]
    
    # Sea creature spawn points
    creature_positions = [
        (2000, -7000, "kraken_spawn"),
        (8000, -5200, "shark_spawn"),
        (5200, -9000, "whale_spawn"),
    ]
    
    for name in pirate_positions + creature_positions:
        x, z, spawn_name = name
        
        # Base object
        monster_data = write_object(monster_data,
            spawn_name, 0, 0, 0, 0,
            0, 0, [0, 0, 0, 1],
            [x * 100, z * 100, 0], [1, 1, 1])
        
        # Spawn name
        monster_data = write_u8_string(monster_data, spawn_name)
        
        # Basic spawns (1 monster type per spawn)
        monster_data += struct.pack('<I', 1)  # basic_count
        monster_data = write_u8_string(monster_data, "")  # monster_name (empty)
        monster_data += struct.pack('<I', 1)  # monster_id
        monster_data += struct.pack('<I', 3)  # monster_count
        
        # Tactic spawns (none)
        monster_data += struct.pack('<I', 0)  # tactic_count
        
        # Spawn parameters
        monster_data += struct.pack('<I', 300)  # interval (300 seconds)
        monster_data += struct.pack('<I', 5)    # limit_count
        monster_data += struct.pack('<I', 100)  # range
        monster_data += struct.pack('<I', 0)    # tactic_points
    
    block_data[MONSTER_SPAWN] = monster_data
    
    # === NPCs (dock masters, merchants) ===
    npc_data = struct.pack('<I', 4)  # 4 NPCs
    
    npc_positions = [
        (5200, -5100, "dock_master_1", "DockMaster"),
        (5200, -5300, "merchant_1", "Merchant"),
        (2000, -1900, "dock_master_2", "DockMaster"),
        (8000, -8100, "merchant_2", "Merchant"),
    ]
    
    for x, z, npc_name, quest_file in npc_positions:
        npc_data = write_object(npc_data,
            npc_name, 0, 0, 0, 0,
            0, 0, [0, 0, 0, 1],
            [x * 100, z * 100, 100], [1, 1, 1])
        npc_data += struct.pack('<I', 0)  # ai_id
        npc_data = write_u8_string(npc_data, quest_file)
    
    block_data[NPC] = npc_data
    
    # === Decorative objects (lighthouses, rocks, etc.) ===
    deco_data = struct.pack('<I', 8)  # 8 deco objects
    
    deco_positions = [
        (5200, -5000, "lighthouse_1"),
        (2000, -1800, "lighthouse_2"),
        (8000, -8200, "lighthouse_3"),
        (3000, -3000, "rock_1"),
        (7000, -7000, "rock_2"),
        (5200, -2000, "rock_3"),
        (5200, -8000, "rock_4"),
        (3500, -6900, "rock_5"),
    ]
    
    for x, z, deco_name in deco_positions:
        deco_data = write_object(deco_data,
            deco_name, 0, 0, 0, 0,
            0, 0, [0, 0, 0, 1],
            [x * 100, z * 100, 100], [1, 1, 1])
    
    block_data[DECO_OBJECT] = deco_data
    
    # === Write the IFO file ===
    # Header: block_count + block_type/offset pairs
    block_types = sorted(block_data.keys())
    block_count = len(block_types)
    
    header_size = 4 + (block_count * 8)  # count + (type + offset) * count
    
    data = struct.pack('<I', block_count)
    
    current_offset = header_size
    for block_type in block_types:
        data += struct.pack('<I', block_type)
        data += struct.pack('<I', current_offset)
        current_offset += len(block_data[block_type])
    
    # Write block data
    for block_type in block_types:
        data += block_data[block_type]
    
    with open(f"{ZONE_DIR}/ifo/0.ifo", 'wb') as f:
        f.write(data)
    print(f"Generated 0.ifo ({len(data)} bytes, {block_count} blocks)")

def generate_zone_manifest():
    """Generate zone manifest JSON for documentation."""
    manifest = {
        "zone_id": ZONE_ID,
        "zone_name": ZONE_NAME,
        "zone_type": "sailing",
        "dimensions": {
            "width_m": ZONE_SIZE,
            "height_m": ZONE_SIZE,
            "grid_size": 257,
        },
        "islands": [
            {"index": i, "center": [cx, cz], "radius_m": r, "peak_height_m": h}
            for i, (cx, cz, r, h) in enumerate(ISLANDS)
        ],
        "docks": [
            {"index": i, "island": idx, "offset": [ox, oz], "rotation_deg": rot}
            for i, (idx, ox, oz, rot) in enumerate(DOCKS)
        ],
        "features": [
            "Water planes covering entire zone",
            "17 islands of varying sizes",
            "10 dock positions with warp points",
            "6 monster spawn points (pirate ships, sea creatures)",
            "4 NPCs (dock masters, merchants)",
            "8 decorative objects (lighthouses, rocks)",
        ],
    }
    
    with open(f"{ZONE_DIR}/zone_manifest.json", 'w') as f:
        json.dump(manifest, f, indent=2)
    print("Generated zone_manifest.json")

def main():
    print(f"=== Generating Zone {ZONE_ID} ({ZONE_NAME}) ===")
    ensure_dirs()
    
    generate_zon()
    generate_him()
    generate_til()
    generate_ifo()
    generate_zone_manifest()
    
    print(f"\nZone {ZONE_ID} generated successfully in {ZONE_DIR}")
    print("Files:")
    for root, dirs, files in os.walk(ZONE_DIR):
        for f in files:
            path = os.path.join(root, f)
            size = os.path.getsize(path)
            print(f"  {path} ({size} bytes)")

if __name__ == "__main__":
    main()

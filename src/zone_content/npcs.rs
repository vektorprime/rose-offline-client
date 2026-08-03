//! Client-side dock NPC spawner for the ocean zone (zone 200).
//!
//! Zone 200's IFO contains no NPC entries, so the docks and islands get their
//! boat vendor, ferryman, quest giver and traders from this system instead.
//! Spawned NPCs are parented to the zone entity so they despawn automatically
//! when the zone unloads, and their transforms are zone-local (the zone entity
//! is positioned at world (5200, 0, -5200), which is the origin of the map's
//! signed coordinate space).

use bevy::prelude::*;

use rose_data::NpcId;
use rose_game_common::components::{
    AbilityValues, HealthPoints, Level, MoveMode, MoveSpeed, Npc, StatusEffects,
    StatusEffectsRegen, Team,
};

use crate::{
    components::{
        ClientEntity, ClientEntityId, ClientEntityName, ClientEntityType, Command,
        FacingDirection, NextCommand, Position, Zone,
    },
    events::ZoneEvent,
    resources::{CurrentZone, GameData},
    systems::OCEAN_ZONE_ID,
    zone_loader::ZoneLoaderAsset,
};

/// Map center in game cm (unsigned). The zone entity transform (5200, 0, -5200)
/// places this point at the world origin, so zone-local meters for a game cm
/// position (x, y, z) are ((x - CENTER)/100, z/100, -(y - CENTER)/100).
const ZONE_CENTER_CM: f32 = 520000.0;

/// Base for synthetic client entity ids used by local dock NPCs.
const DOCK_NPC_ENTITY_ID_BASE: usize = 0x4000_0000;

/// Marker component identifying entities spawned by this system.
#[derive(Component)]
pub struct DockNpc;

/// One dock NPC definition: id from LIST_NPC.STB, spawn position in game cm
/// (z is filled from the terrain height at spawn time).
struct DockNpcSpawn {
    npc_id: u16,
    name: &'static str,
    position_cm: Vec3,
}

const DOCK_NPCS: &[DockNpcSpawn] = &[
    // Boat vendor on the main island, ~40 m west of the player spawn point.
    DockNpcSpawn {
        npc_id: 1006, // [Arumic Merchant] Tryteh
        name: "Tryteh",
        position_cm: Vec3::new(516000.0, 519000.0, 0.0),
    },
    // Ferryman on the north shore of the main island, right at the waterline.
    DockNpcSpawn {
        npc_id: 1037, // [Old Fisherman] Myad
        name: "Myad",
        position_cm: Vec3::new(520000.0, 527000.0, 0.0),
    },
    // Quest giver on the main island peak, above the marina.
    DockNpcSpawn {
        npc_id: 1014, // [Guide] Lena
        name: "Lena",
        position_cm: Vec3::new(506000.0, 519000.0, 0.0),
    },
    // Trader on the east island (island measured from HIM at (5630, 4847)).
    DockNpcSpawn {
        npc_id: 1013, // [Tavern Owner] Sharlin
        name: "Sharlin",
        position_cm: Vec3::new(560000.0, 484700.0, 0.0),
    },
    // Weapon seller on the west island (island measured from HIM at (4633, 4914)).
    DockNpcSpawn {
        npc_id: 1008, // [Weapon Seller] Raffle
        name: "Raffle",
        position_cm: Vec3::new(465000.0, 493000.0, 0.0),
    },
];

fn spawn_dock_npc(
    commands: &mut Commands,
    game_data: &GameData,
    zone_entity: Entity,
    spawn: &DockNpcSpawn,
    index: usize,
    terrain_height_cm: f32,
) {
    let Some(npc_id) = NpcId::new(spawn.npc_id) else {
        return;
    };

    // Standing height: terrain + small offset so feet do not sink into the ground.
    let height_cm = terrain_height_cm + 10.0;
    let position = Vec3::new(spawn.position_cm.x, spawn.position_cm.y, height_cm);

    // Zone-local transform: game cm -> meters relative to the zone entity.
    let transform = Transform::from_xyz(
        (position.x - ZONE_CENTER_CM) / 100.0,
        position.z / 100.0,
        -(position.y - ZONE_CENTER_CM) / 100.0,
    );

    let status_effects = StatusEffects::default();
    let Some(ability_values) = game_data
        .ability_value_calculator
        .calculate_npc(npc_id, &status_effects, None, None)
    else {
        return;
    };

    let npc_entity = commands
        .spawn((
            Npc::new(npc_id, 0),
            Command::with_stop(),
            NextCommand::with_stop(),
            Team::default_npc(),
            HealthPoints::new(ability_values.get_max_health()),
            MoveMode::Walk,
            Position::new(position),
            ability_values.clone(),
            Level::new(ability_values.get_level() as u32),
            MoveSpeed::new(ability_values.get_move_speed(&MoveMode::Walk)),
            status_effects,
            StatusEffectsRegen::new(),
        ))
        .insert((
            FacingDirection::default(),
            ClientEntity::new(
                ClientEntityId(DOCK_NPC_ENTITY_ID_BASE + index),
                ClientEntityType::Npc,
            ),
            ClientEntityName::new(spawn.name.to_string()),
            DockNpc,
            transform,
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    commands.entity(zone_entity).add_child(npc_entity);
}

/// Spawns the dock NPCs once when zone 200 loads. Falls back to spawning if the
/// ZoneEvent::Loaded event was missed but the zone entity already exists, and
/// resets the guard when the zone has been unloaded so re-entry respawns them.
pub fn spawn_dock_npcs_system(
    mut commands: Commands,
    mut zone_events: MessageReader<ZoneEvent>,
    zone_query: Query<(Entity, &Zone)>,
    mut spawned: Local<bool>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    game_data: Res<GameData>,
) {
    let mut zone_loaded = false;
    for event in zone_events.read() {
        if matches!(event, ZoneEvent::Loaded(zone_id) if zone_id.get() == OCEAN_ZONE_ID) {
            zone_loaded = true;
        }
    }

    let zone_entity = zone_query
        .iter()
        .find(|(_, zone)| zone.id.get() == OCEAN_ZONE_ID)
        .map(|(entity, _)| entity);

    match (zone_loaded, *spawned, zone_entity) {
        // Zone was unloaded: reset the guard so a future load respawns.
        (false, true, None) => *spawned = false,
        // Already spawned: do nothing.
        (_, true, _) => {}
        // Spawn on the Loaded event, or on the fallback path if the event was missed.
        (_, false, Some(zone_entity)) => {
            let zone_data = current_zone
                .as_ref()
                .and_then(|zone| zone_loader_assets.get(&zone.handle));

            for (index, spawn) in DOCK_NPCS.iter().enumerate() {
                let terrain_height_cm = zone_data
                    .map(|data| data.get_terrain_height(spawn.position_cm.x, spawn.position_cm.y))
                    .unwrap_or(150.0);
                spawn_dock_npc(
                    &mut commands,
                    &game_data,
                    zone_entity,
                    spawn,
                    index,
                    terrain_height_cm,
                );
            }

            *spawned = true;
        }
        (_, false, None) => {}
    }
}

//! Temporary memory diagnostics.
//!
//! Logs entity counts and key asset counts every 30 seconds (gated by the
//! `memory_diagnostics_due` run condition) so memory leaks can
//! be pinpointed empirically (which counter keeps growing?). Remove this module
//! once the leak is confirmed fixed.

use std::time::{Duration, Instant};

use bevy::{
    asset::Assets,
    ecs::{
        entity::Entities,
        message::Messages,
        system::SystemParam,
    },
    pbr::ExtendedMaterial,
    prelude::*,
    render::storage::ShaderBuffer,
};

use crate::{
    animation::ZmoAsset,
    audio::AudioSource,
    components::{Bird, ChatBubbleEntity, ClientEntity, DamageNumber, Fish, NameTag, Zone, ZoneObject},
    events::{ChatBubbleEvent, ZoneEvent},
    render::{ParticleMaterial, RoseObjectExtension},
    zone_loader::ZoneLoaderAsset,
};

/// Bundled time/state/entity params. Each derived SystemParam struct counts as a
/// single system parameter, so bundling keeps the system within Bevy's 20-parameter
/// limit (this system would otherwise expand to 22 parameters).
#[derive(SystemParam)]
pub struct MemoryDiagMeta<'w> {
    time: Res<'w, Time>,
    entities: &'w Entities,
}

/// Bundled message resources to keep the system parameter count under the limit.
#[derive(SystemParam)]
pub struct MemoryDiagEvents<'w> {
    zone_events: Res<'w, Messages<ZoneEvent>>,
    chat_bubble_events: Res<'w, Messages<ChatBubbleEvent>>,
}

/// Bundled asset-collection params to stay within Bevy's 16-parameter limit.
#[derive(SystemParam)]
pub struct MemoryDiagAssets<'w> {
    meshes: Res<'w, Assets<Mesh>>,
    images: Res<'w, Assets<Image>>,
    storage_buffers: Res<'w, Assets<ShaderBuffer>>,
    object_materials: Res<'w, Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    standard_materials: Res<'w, Assets<StandardMaterial>>,
    particle_materials: Res<'w, Assets<ParticleMaterial>>,
    zmo_assets: Res<'w, Assets<ZmoAsset>>,
    audio_sources: Res<'w, Assets<AudioSource>>,
    zone_loader_assets: Res<'w, Assets<ZoneLoaderAsset>>,
}

/// Bundled query params to stay within Bevy's 16-parameter limit.
#[derive(SystemParam)]
pub struct MemoryDiagQueries<'w, 's> {
    bird_query: Query<'w, 's, (), With<Bird>>,
    zone_query: Query<'w, 's, (Entity, &'static Zone, Option<&'static Children>)>,
    client_entity_query: Query<'w, 's, (), With<ClientEntity>>,
    zone_object_query: Query<'w, 's, (), With<ZoneObject>>,
    fish_query: Query<'w, 's, (), With<Fish>>,
    name_tag_query: Query<'w, 's, (), With<NameTag>>,
    chat_bubble_query: Query<'w, 's, (), With<ChatBubbleEntity>>,
    damage_number_query: Query<'w, 's, (), With<DamageNumber>>,
}

/// Run condition for [`memory_diagnostics_system`]: true on the first frame,
/// then once every 30 seconds.
pub fn memory_diagnostics_due(mut last_log: Local<Option<Instant>>) -> bool {
    let now = Instant::now();
    if last_log.is_some_and(|last| now.duration_since(last) < Duration::from_secs(30)) {
        return false;
    }
    *last_log = Some(now);
    true
}

pub fn memory_diagnostics_system(
    meta: MemoryDiagMeta,
    assets: MemoryDiagAssets,
    queries: MemoryDiagQueries,
    events: MemoryDiagEvents,
) {
    let zones: Vec<String> = queries
        .zone_query
        .iter()
        .map(|(entity, zone, children)| {
            let child_count = children.map(|c| c.len()).unwrap_or(0);
            format!("{}({}:{}k)", zone.id.get(), entity.index(), child_count / 1000)
        })
        .collect();

    log::info!(
        "[MEMORY DIAG] elapsed={:.0}s fps={:.0} alive_entities={} allocated_slots={} \
         zones=[{}] birds={} client_entities={} zone_objects={} fish={} name_tags={} chat_bubbles={} \
         meshes={} images={} shader_storage_buffers={} object_materials={} standard_materials={} \
         particle_materials={} damage_numbers={} zmo_assets={} audio_sources={} \
         zone_assets={} zone_events_pending={} chat_bubble_events_pending={}",
        meta.time.elapsed_secs(),
        1.0 / meta.time.delta_secs().max(0.0001),
        meta.entities.count_spawned(),
        meta.entities.len(),
        zones.join(","),
        queries.bird_query.iter().count(),
        queries.client_entity_query.iter().count(),
        queries.zone_object_query.iter().count(),
        queries.fish_query.iter().count(),
        queries.name_tag_query.iter().count(),
        queries.chat_bubble_query.iter().count(),
        assets.meshes.len(),
        assets.images.len(),
        assets.storage_buffers.len(),
        assets.object_materials.len(),
        assets.standard_materials.len(),
        assets.particle_materials.len(),
        queries.damage_number_query.iter().count(),
        assets.zmo_assets.len(),
        assets.audio_sources.len(),
        assets.zone_loader_assets.len(),
        events.zone_events.len(),
        events.chat_bubble_events.len(),
    );
}

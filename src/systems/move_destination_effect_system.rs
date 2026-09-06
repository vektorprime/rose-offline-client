use bevy::{
    pbr::{ExtendedMaterial, StandardMaterial},
    prelude::{
        AssetServer, Assets, Commands, Entity, GlobalTransform, InheritedVisibility, Local, Mesh,
        MessageReader, Query, Res, ResMut, Transform, Vec3, ViewVisibility, Visibility, With,
    },
    material::AlphaMode,
};
use rose_data::EffectFileId;

use crate::{
    components::{PlayerCharacter, Position},
    effect_loader::{spawn_effect, EffectCache},
    events::MoveDestinationEffectEvent,
    render::{ParticleMaterial, RoseEffectExtension},
    resources::{GameData, VfsResource},
};

#[derive(Default)]
pub struct MoveDestinationEffectSystemState {
    pub last_effect_entity: Option<Entity>,
    pub last_position: Option<Vec3>,
}

pub fn move_destination_effect_system(
    mut commands: Commands,
    mut state: Local<MoveDestinationEffectSystemState>,
    mut events: MessageReader<MoveDestinationEffectEvent>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    vfs_resource: Res<VfsResource>,
    effect_cache: Res<EffectCache>,
    mut effect_mesh_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
    >,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut storage_buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    player_query: Query<&Position, With<PlayerCharacter>>,
) {
    for event in events.read() {
        match event {
            MoveDestinationEffectEvent::Show { position } => {
                if let Some(last_effect_entity) = state.last_effect_entity.take() {
                    commands.entity(last_effect_entity).despawn();
                }
                state.last_position = Some(*position);

                if let Some(effect_file_path) = game_data
                    .effect_database
                    .get_effect_file(EffectFileId::new(296).unwrap())
                    .map(|x| x.into())
                {
                    let effect_entity = commands
                        .spawn((
                            // Scaled to ~half: the effect-296 cone reads oversized
                            // at full scale next to the Hanabi ring pilot.
                            Transform::from_translation(*position)
                                .with_scale(Vec3::splat(0.5)),
                            GlobalTransform::default(),
                            Visibility::default(),
                            InheritedVisibility::default(),
                            ViewVisibility::default(),
                        ))
                        .id();
                    state.last_effect_entity = Some(effect_entity);

                    spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        &mut storage_buffers,
                        &mut meshes,
                        effect_file_path,
                        true,
                        Some(effect_entity),
                        Some(&effect_cache),
                        Some(*position),
                    );
                }
            }
            MoveDestinationEffectEvent::Hide => {
                if let Some(last_effect_entity) = state.last_effect_entity.take() {
                    commands.entity(last_effect_entity).despawn();
                }
                state.last_position = None;
            }
        }
    }

    // Dismiss the marker on arrival: Position is centimetres in
    // (x, -z, y) order, Show positions are metres in (x, y, z).
    if let (Some(entity), Some(dest)) = (state.last_effect_entity, state.last_position) {
        let arrived = player_query.iter().next().is_some_and(|p| {
            let player_m = Vec3::new(p.position.x, p.position.z, -p.position.y) / 100.0;
            player_m.distance(dest) < 2.0
        });
        if arrived {
            commands.entity(entity).despawn();
            state.last_effect_entity = None;
            state.last_position = None;
        }
    }
}

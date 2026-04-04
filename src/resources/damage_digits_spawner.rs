use bevy::{
    pbr::MeshMaterial3d,
    prelude::{
        AssetServer, Assets, Commands, Component, GlobalTransform, Handle, Image, Mesh, Mesh3d,
        Resource, Transform, Vec2, Vec3, Vec4, Visibility,
    },
};
use bevy_camera::{
    primitives::Aabb,
    visibility::{InheritedVisibility, NoFrustumCulling, ViewVisibility},
};

use crate::{
    animation::{TransformAnimation, ZmoAsset},
    components::DamageDigits,
    render::{DamageDigitMaterial, DamageDigitRenderData},
};

#[derive(Resource)]
pub struct DamageDigitsSpawner {
    // Store texture handles instead of material handles
    // Materials are created per-entity with their own storage buffers
    pub texture_damage: Handle<Image>,
    pub texture_damage_player: Handle<Image>,
    pub texture_miss: Handle<Image>,
    pub motion: Handle<ZmoAsset>,
    pub mesh: Handle<Mesh>,
}

impl DamageDigitsSpawner {
    pub fn load(asset_server: &AssetServer, meshes: &mut Assets<Mesh>) -> Self {
        log::info!("[DAMAGE_DIGITS_SPAWNER::load] Loading damage digit assets...");

        let texture_damage = asset_server.load("3ddata/effect/special/digitnumber01.dds");
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER::load] Loaded texture_damage: {:?}",
            texture_damage
        );

        let texture_damage_player = asset_server.load("3ddata/effect/special/digitnumber02.dds");
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER::load] Loaded texture_damage_player: {:?}",
            texture_damage_player
        );

        let texture_miss = asset_server.load("3ddata/effect/special/digitnumbermiss.dds");
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER::load] Loaded texture_miss: {:?}",
            texture_miss
        );

        let motion = asset_server.load("3ddata/effect/special/hit_figure_01.zmo");
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER::load] Loaded motion (ZMO): {:?}",
            motion
        );

        let mesh = meshes.add(Mesh::from(bevy::prelude::Rectangle::new(1.0, 1.0)));
        log::info!("[DAMAGE_DIGITS_SPAWNER::load] Created mesh: {:?}", mesh);

        log::info!("[DAMAGE_DIGITS_SPAWNER::load] DamageDigitsSpawner initialized successfully");

        Self {
            texture_damage,
            texture_damage_player,
            texture_miss,
            motion,
            mesh,
        }
    }

    /// Get the appropriate texture handle based on damage type
    pub fn get_texture(&self, damage: u32, is_damage_player: bool) -> Handle<Image> {
        if damage == 0 {
            self.texture_miss.clone()
        } else if is_damage_player {
            self.texture_damage_player.clone()
        } else {
            self.texture_damage.clone()
        }
    }

    pub fn spawn(
        &self,
        commands: &mut Commands,
        global_transform: &GlobalTransform,
        model_height: f32,
        damage: u32,
        is_damage_player: bool,
    ) {
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER] spawn() called with damage={}, is_damage_player={}",
            damage,
            is_damage_player
        );

        let (scale, _, translation) = global_transform.to_scale_rotation_translation();
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER] GlobalTransform: scale={:?}, translation={:?}",
            scale,
            translation
        );

        // Get the appropriate texture
        let texture_handle = self.get_texture(damage, is_damage_player);
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER] Selected texture handle: {:?}",
            texture_handle
        );

        // We need to spawn inside a parent entity for positioning because the ActiveMotion will set the translation absolutely
        // Spawn the child entity first - note: material will be added later by damage_digit_render_system
        // Using chained inserts to avoid tuple length limits
        log::info!("[DAMAGE_DIGITS_SPAWNER] Spawning child entity with DamageDigits, DamageDigitRenderData, PendingDamageDigitMaterial, Mesh3d, TransformAnimation, Transform, GlobalTransform, Visibility components");
        let child_entity = commands
            .spawn((
                DamageDigits { damage },
                DamageDigitRenderData::new(4),
                PendingDamageDigitMaterial {
                    texture: texture_handle.clone(),
                },
                TransformAnimation::once(self.motion.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER] Child entity spawned with id: {:?}",
            child_entity
        );

        // Then spawn the parent and add the child
        let parent_position = translation + Vec3::new(0.0, model_height * scale.y, 0.0);
        log::info!(
            "[DAMAGE_DIGITS_SPAWNER] Parent position: {:?}",
            parent_position
        );
        commands
            .spawn((
                Transform::from_translation(parent_position),
                Visibility::default(),
            ))
            .add_children(&[child_entity]);
        log::info!("[DAMAGE_DIGITS_SPAWNER] Parent entity spawned and child added. Damage digit entity creation complete.");
        // Note: NoFrustumCulling removed due to tuple length limit
    }
}

/// Component marker for entities that need a DamageDigitMaterial created
/// The damage_digit_render_system will create the actual material with storage buffers
#[derive(Component)]
pub struct PendingDamageDigitMaterial {
    pub texture: Handle<Image>,
}

use bevy::{
    pbr::MeshMaterial3d,
    prelude::{
        AssetServer, Assets, Commands, ViewVisibility, InheritedVisibility, GlobalTransform, Handle,
        Mesh3d, Resource, Transform, Vec3, Visibility, Mesh, Vec4, Vec2, Image, Component,
    },
    render::{primitives::Aabb, view::NoFrustumCulling},
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
    pub fn load(
        asset_server: &AssetServer,
        meshes: &mut Assets<Mesh>,
    ) -> Self {
        Self {
            texture_damage: asset_server.load("3ddata/effect/special/digitnumber01.dds"),
            texture_damage_player: asset_server.load("3ddata/effect/special/digitnumber02.dds"),
            texture_miss: asset_server.load("3ddata/effect/special/digitnumbermiss.dds"),
            motion: asset_server.load("3ddata/effect/special/hit_figure_01.zmo"),
            mesh: meshes.add(Mesh::from(bevy::prelude::Rectangle::new(1.0, 1.0))),
        }
    }

    /// Get the appropriate texture handle based on damage type
    pub fn get_texture(&self, damage: u32, is_damage_player: bool) -> Handle<Image> {
        if damage == 0 {
            self.texture_miss.clone_weak()
        } else if is_damage_player {
            self.texture_damage_player.clone_weak()
        } else {
            self.texture_damage.clone_weak()
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
        let (scale, _, translation) = global_transform.to_scale_rotation_translation();

        // We need to spawn inside a parent entity for positioning because the ActiveMotion will set the translation absolutely
        // Spawn the child entity first - note: material will be added later by damage_digit_render_system
        // Using chained inserts to avoid tuple length limits
        let child_entity = commands
            .spawn(DamageDigits { damage })
            .insert(DamageDigitRenderData::new(4))
            .insert(PendingDamageDigitMaterial {
                texture: self.get_texture(damage, is_damage_player),
            })
            .insert(Mesh3d(self.mesh.clone_weak()))
            .insert(TransformAnimation::once(self.motion.clone_weak()))
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .insert(Visibility::default())
            .insert(InheritedVisibility::default())
            .insert(ViewVisibility::default())
            .id();

        // Then spawn the parent and add the child
        commands
            .spawn((
                Transform::from_translation(
                    translation + Vec3::new(0.0, model_height * scale.y, 0.0),
                ),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .add_children(&[child_entity]);
            // Note: NoFrustumCulling removed due to tuple length limit
     }
}

/// Component marker for entities that need a DamageDigitMaterial created
/// The damage_digit_render_system will create the actual material with storage buffers
#[derive(Component)]
pub struct PendingDamageDigitMaterial {
    pub texture: Handle<Image>,
}

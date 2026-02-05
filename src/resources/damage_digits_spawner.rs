use bevy::{
    pbr::MeshMaterial3d,
    prelude::{
        AssetServer, Assets, BuildChildren, Commands, ViewVisibility, InheritedVisibility, GlobalTransform, Handle,
        Mesh3d, Resource, Transform, Vec3, Visibility, Mesh, Vec4, Vec2,
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
    pub texture_damage: Handle<DamageDigitMaterial>,
    pub texture_damage_player: Handle<DamageDigitMaterial>,
    pub texture_miss: Handle<DamageDigitMaterial>,
    pub motion: Handle<ZmoAsset>,
    pub mesh: Handle<Mesh>,
}

    impl DamageDigitsSpawner {
    pub fn load(
        asset_server: &AssetServer,
        damage_digit_materials: &mut Assets<DamageDigitMaterial>,
        meshes: &mut Assets<Mesh>,
    ) -> Self {
        Self {
            texture_damage: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBER01.DDS"),
                positions: Vec4::default(),
                sizes: Vec4::default(),
                uvs: Vec4::default(),
            }),
            texture_damage_player: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBER02.DDS"),
                positions: Vec4::default(),
                sizes: Vec4::default(),
                uvs: Vec4::default(),
            }),
            texture_miss: damage_digit_materials.add(DamageDigitMaterial {
                texture: asset_server.load("3DDATA/EFFECT/SPECIAL/DIGITNUMBERMISS.DDS"),
                positions: Vec4::default(),
                sizes: Vec4::default(),
                uvs: Vec4::default(),
            }),
            motion: asset_server.load("3DDATA/EFFECT/SPECIAL/HIT_FIGURE_01.ZMO"),
            mesh: meshes.add(Mesh::from(bevy::prelude::Rectangle::new(1.0, 1.0))),
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
        // Spawn the child entity first
        let child_entity = commands.spawn((
            DamageDigits { damage },
            DamageDigitRenderData::new(4),
            MeshMaterial3d(if damage == 0 {
                self.texture_miss.clone_weak()
            } else if is_damage_player {
                self.texture_damage_player.clone_weak()
            } else {
                self.texture_damage.clone_weak()
            }),
            Mesh3d(self.mesh.clone_weak()),
            TransformAnimation::once(self.motion.clone_weak()),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();

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

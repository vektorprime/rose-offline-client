use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    light::{NotShadowCaster, NotShadowReceiver},
    math::{Affine2, Mat2},
    mesh::{Mesh, PrimitiveTopology},
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{
        AssetServer, Assets, Commands, GlobalTransform, Handle, Image, Mesh3d, Resource,
        Transform, Vec2, Vec3, Visibility,
    },
    material::AlphaMode,
};

use crate::components::DamageNumber;

/// World-space damage digits as plain Bevy meshes.
///
/// Replaces the old custom `DamageDigitMaterial` pipeline (storage buffers +
/// procedural WGSL + ZMO motion). Each digit is a quad with a shared mesh and
/// a cached `StandardMaterial` slicing one cell out of the original ROSE DDS
/// digit strips via `uv_transform`. Everything renders in `Transparent3d`,
/// so — unlike `Text2d`/`Sprite`, which Bevy only draws for `Camera2d` views
/// (`extract_core_2d_camera_phases` in `bevy_core_pipeline`) — digits show up
/// under this game's `Camera3d`.
#[derive(Resource)]
pub struct DamageDigitsSpawner {
    quad: Handle<Mesh>,
    damage: Vec<Handle<StandardMaterial>>,
    damage_player: Vec<Handle<StandardMaterial>>,
    miss: Vec<Handle<StandardMaterial>>,
}

/// Quad size in world units (matches the old 0.4m digit quads).
const DIGIT_SIZE: f32 = 0.4;
/// Center-to-center digit spacing (same seamless tiling as the old shader).
const DIGIT_SPACING: f32 = 0.4;

fn digit_material(texture: Handle<Image>, offset_x: f32, width: f32) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(texture),
        // Unlit + fogless: digits must stay crisp like the old custom shader,
        // which did raw texture sampling with no lighting or fog.
        unlit: true,
        fog_enabled: false,
        alpha_mode: AlphaMode::Blend,
        uv_transform: Affine2 {
            matrix2: Mat2::from_diagonal(Vec2::new(width, 1.0)),
            translation: Vec2::new(offset_x, 0.0),
        },
        ..Default::default()
    }
}

/// 0.4m quad in the XY plane facing +Z, built explicitly so UV orientation is
/// known: top vertices sample v=0 (image top), matching the old digit shader
/// which mapped quad-top to the strip's v=0 edge.
fn build_digit_quad() -> Mesh {
    let h = DIGIT_SIZE / 2.0;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-h, -h, 0.0],
            [h, -h, 0.0],
            [h, h, 0.0],
            [-h, -h, 0.0],
            [h, h, 0.0],
            [-h, h, 0.0],
        ],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 0.0, 1.0]; 6],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ],
    )
}

impl DamageDigitsSpawner {
    pub fn load(
        asset_server: &AssetServer,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let texture_damage = asset_server.load("3ddata/effect/special/digitnumber01.dds");
        let texture_damage_player =
            asset_server.load("3ddata/effect/special/digitnumber02.dds");
        let texture_miss = asset_server.load("3ddata/effect/special/digitnumbermiss.dds");
        let quad = meshes.add(build_digit_quad());

        // Digit strips hold 10 cells; the miss strip holds 4 ("MISS" slices,
        // same split the old shader used).
        let damage = (0..10)
            .map(|i| {
                materials.add(digit_material(
                    texture_damage.clone(),
                    i as f32 / 10.0,
                    1.0 / 10.0,
                ))
            })
            .collect();
        let damage_player = (0..10)
            .map(|i| {
                materials.add(digit_material(
                    texture_damage_player.clone(),
                    i as f32 / 10.0,
                    1.0 / 10.0,
                ))
            })
            .collect();
        let miss = (0..4)
            .map(|i| {
                materials.add(digit_material(
                    texture_miss.clone(),
                    i as f32 / 4.0,
                    1.0 / 4.0,
                ))
            })
            .collect();

        log::info!("[DAMAGE_DIGITS_SPAWNER] Damage digit quads + materials ready");

        Self {
            quad,
            damage,
            damage_player,
            miss,
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

        // Above the head, with a small random offset so simultaneous hits
        // don't stack exactly on top of each other.
        let jitter = (rand::random::<f32>() - 0.5) * 0.6;
        let position = translation + Vec3::new(jitter, model_height * scale.y + 0.3, 0.0);

        // (material, x offset in digit units). LSD-first layout mirrors the
        // old shader: least significant digit on the right.
        let mut digits: Vec<(Handle<StandardMaterial>, f32)> = Vec::new();
        if damage == 0 {
            for i in 0..4 {
                digits.push((self.miss[i].clone(), -1.5 + i as f32));
            }
        } else {
            let mut count = 0;
            let mut rest = damage;
            while rest > 0 {
                count += 1;
                rest /= 10;
            }
            let number_offset = (count - 1) as f32 / 2.0;
            let mut digit_offset = 0.0;
            let mut rest = damage;
            let mats = if is_damage_player {
                &self.damage_player
            } else {
                &self.damage
            };
            while rest > 0 {
                let digit = (rest % 10) as usize;
                digits.push((mats[digit].clone(), number_offset - digit_offset));
                digit_offset += 1.0;
                rest /= 10;
            }
        }

        log::info!(
            "[DAMAGE_NUMBER] spawn damage={} digits={} at {:?}",
            damage,
            digits.len(),
            position
        );

        let quad = self.quad.clone();
        commands
            .spawn((
                Transform::from_translation(position),
                Visibility::default(),
                DamageNumber::new(1.1, 1.6),
            ))
            .with_children(|parent| {
                for (material, x) in digits {
                    parent.spawn((
                        Mesh3d(quad.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(Vec3::new(x * DIGIT_SPACING, 0.0, 0.0)),
                        Visibility::default(),
                        NoFrustumCulling,
                        NotShadowCaster,
                        NotShadowReceiver,
                    ));
                }
            });
    }
}

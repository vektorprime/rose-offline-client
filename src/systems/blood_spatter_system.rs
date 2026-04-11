//! Blood spatter effect systems for combat visual feedback.
//!
//! This module implements blood spatter decals that appear on terrain when
//! entities are killed. The system uses Bevy's ForwardDecal for rendering.

use bevy::{
    asset::RenderAssetUsages,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
    prelude::*,
    render::render_resource::{Extent3d, Face, TextureDimension, TextureFormat},
};

use crate::{
    components::{BloodSpatter, Dead, DeathBloodHandled, ModelHeight},
    events::{BloodEffectEvent, BloodImpactProfile},
    resources::{
        BloodDecalAtlas, BloodEffectConfig, BloodEffectDiagnostics, BloodEffectRuntime,
        ClientEntityList,
    },
};

const SPATTER_TEXTURE_VARIANTS: usize = 8;

fn hash01(x: f32, y: f32, seed: f32) -> f32 {
    let v = (x * 12.9898 + y * 78.233 + seed * 37.719).sin() * 43_758.5453;
    v - v.floor()
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let len_sq = value.length_squared();
    if len_sq > 1e-6 {
        value / len_sq.sqrt()
    } else {
        fallback
    }
}

fn build_spatter_transform(position: Vec3, normal: Vec3, size: f32, rotation: f32) -> Transform {
    let surface_normal = normalize_or(normal, Vec3::Y);
    let align_to_surface = Quat::from_rotation_arc(Vec3::Y, surface_normal);
    let spin_on_surface = Quat::from_axis_angle(surface_normal, rotation);

    Transform::from_translation(position + surface_normal * 0.01)
        .with_rotation(spin_on_surface * align_to_surface)
        .with_scale(Vec3::new(size, size, 1.0))
}

fn pick_spatter_texture(atlas: &BloodDecalAtlas) -> Option<Handle<Image>> {
    if atlas.spatter_textures.is_empty() {
        None
    } else {
        let idx = rand::random::<usize>() % atlas.spatter_textures.len();
        atlas.spatter_textures.get(idx).cloned()
    }
}

fn profile_count_multiplier(profile: BloodImpactProfile) -> f32 {
    match profile {
        BloodImpactProfile::Slash => 1.0,
        BloodImpactProfile::Pierce => 0.8,
        BloodImpactProfile::Blunt => 0.65,
        BloodImpactProfile::SkillMagic => 1.25,
        BloodImpactProfile::Projectile => 0.9,
    }
}

fn profile_alpha_multiplier(profile: BloodImpactProfile) -> f32 {
    match profile {
        BloodImpactProfile::Slash => 1.0,
        BloodImpactProfile::Pierce => 1.1,
        BloodImpactProfile::Blunt => 0.85,
        BloodImpactProfile::SkillMagic => 0.9,
        BloodImpactProfile::Projectile => 1.0,
    }
}

fn profile_spread_multiplier(profile: BloodImpactProfile) -> f32 {
    match profile {
        BloodImpactProfile::Slash => 1.0,
        BloodImpactProfile::Pierce => 0.8,
        BloodImpactProfile::Blunt => 1.2,
        BloodImpactProfile::SkillMagic => 1.35,
        BloodImpactProfile::Projectile => 0.9,
    }
}

/// Ensure blood textures are generated once and reused.
pub fn initialize_blood_decal_atlas_system(
    mut atlas: ResMut<BloodDecalAtlas>,
    mut images: ResMut<Assets<Image>>,
) {
    if atlas.spatter_textures.is_empty() {
        for variant in 0..SPATTER_TEXTURE_VARIANTS {
            atlas
                .spatter_textures
                .push(create_blood_texture_variant(&mut images, variant));
        }
    }

    if atlas.wound_textures.is_empty() {
        atlas
            .wound_textures
            .push(create_blood_texture_variant(&mut images, SPATTER_TEXTURE_VARIANTS + 1));
    }
}

/// System that listens for entities being marked as Dead and spawns blood spatter events.
///
/// This system queries for entities that just had the `Dead` component added
/// and sends [`BloodEffectEvent::SpawnSpatter`] events to trigger visual effects.
pub fn blood_spatter_on_death_system(
    mut blood_events: MessageWriter<BloodEffectEvent>,
    query_dead: Query<
        &GlobalTransform,
        (Added<Dead>, With<ModelHeight>, Without<DeathBloodHandled>),
    >,
    config: Res<BloodEffectConfig>,
) {
    if !config.enable_blood {
        return;
    }

    for transform in query_dead.iter() {
        let position = transform.translation();

        // Spawn blood spatter at feet position
        blood_events.write(BloodEffectEvent::kill_spatter_with_profile(
            position,
            Vec3::Y,
            0, // Final blow damage already applied
            Vec3::Y,
            BloodImpactProfile::Slash,
        ));
    }
}

/// System that processes blood effect events and spawns spatter decals.
///
/// This system handles:
/// - [`BloodEffectEvent::SpawnSpatter`] - Creates forward decal entities for blood
pub fn blood_spatter_spawn_system(
    mut commands: Commands,
    mut blood_events: MessageReader<BloodEffectEvent>,
    config: Res<BloodEffectConfig>,
    query_spatters: Query<
        (
            Entity,
            &BloodSpatter,
            &MeshMaterial3d<ForwardDecalMaterial<StandardMaterial>>,
        ),
        With<BloodSpatter>,
    >,
    query_transform: Query<&GlobalTransform>,
    client_entity_list: Res<ClientEntityList>,
    atlas: Res<BloodDecalAtlas>,
    mut decal_materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
    mut runtime: ResMut<BloodEffectRuntime>,
    mut diagnostics: ResMut<BloodEffectDiagnostics>,
) {
    if !config.enable_blood {
        blood_events.clear();
        return;
    }

    // Check if we need to enforce spatter limit
    let mut active_spatter_count = query_spatters.iter().filter(|(_, spatter, _)| spatter.active).count();
    let max_spatters = config.max_spatters;
    let mut frame_spawn_budget = config.effective_spatter_spawn_budget();

    for event in blood_events.read() {
        if let BloodEffectEvent::SpawnSpatter {
            position,
            normal,
            impact_direction,
            damage_amount,
            is_kill,
            profile,
        } = event
        {
            diagnostics.spatter_events = diagnostics.spatter_events.saturating_add(1);

            // Determine spatter count based on event type
            let base_spatter_count = if *is_kill {
                config.effective_kill_spatter_count()
            } else {
                config.effective_hit_spatter_count()
            };

            let profile_mult = profile_count_multiplier(*profile);
            let lod_scale = client_entity_list
                .player_entity
                .and_then(|player| query_transform.get(player).ok())
                .map(|player_transform| {
                    let distance = player_transform.translation().distance(*position);
                    config.distance_lod_scale(distance)
                })
                .unwrap_or(1.0);

            let spatter_count = ((base_spatter_count as f32) * profile_mult * lod_scale)
                .round()
                .max(0.0) as usize;

            if frame_spawn_budget == 0 {
                break;
            }

            let (min_size_base, max_size_base) = config.effective_spatter_size_range();
            let min_size = (min_size_base * lod_scale).max(0.08);
            let max_size = (max_size_base * lod_scale).max(min_size);
            let base_normal = normalize_or(*normal, Vec3::Y);
            let tangent = normalize_or(base_normal.cross(Vec3::Y), Vec3::X);
            let bitangent = normalize_or(base_normal.cross(tangent), Vec3::Z);

            let impact_dir = normalize_or(*impact_direction, Vec3::Y);
            let planar_impact = normalize_or(
                impact_dir - base_normal * impact_dir.dot(base_normal),
                tangent,
            );

            let damage_alpha_scale = ((*damage_amount as f32) / 250.0).clamp(0.35, 1.35);

            // Spawn spatter decals
            for i in 0..spatter_count {
                // Stop if we hit the limit
                if active_spatter_count >= max_spatters {
                    if let Some((oldest_entity, oldest_spatter, _)) = query_spatters
                        .iter()
                        .filter(|(_, spatter, _)| spatter.active)
                        .min_by(|a, b| a.1.lifetime.total_cmp(&b.1.lifetime))
                    {
                        commands.entity(oldest_entity).insert((
                            Visibility::Hidden,
                            BloodSpatter {
                                active: false,
                                ..oldest_spatter.clone()
                            },
                        ));
                        runtime.spatter_pool.push(oldest_entity);
                        diagnostics.pooled_spatters_returned =
                            diagnostics.pooled_spatters_returned.saturating_add(1);
                        active_spatter_count = active_spatter_count.saturating_sub(1);
                    } else {
                        break;
                    }
                }

                if frame_spawn_budget == 0 {
                    break;
                }

                // Biased directional distribution from impact vector + random spread.
                let directional_scale = if *is_kill { 1.1 } else { 0.85 };
                let radial = rand::random::<f32>()
                    * config.spatter_radius
                    * profile_spread_multiplier(*profile);
                let directional = planar_impact * (radial * directional_scale);
                let jitter_t = (rand::random::<f32>() - 0.5) * config.spatter_radius * 0.7;
                let jitter_b = (rand::random::<f32>() - 0.5) * config.spatter_radius * 0.7;
                let random_offset = tangent * jitter_t + bitangent * jitter_b;

                let spatter_pos = *position + directional + random_offset;

                // Random size within range
                let size = min_size + rand::random::<f32>() * (max_size - min_size);

                // Random rotation
                let rotation = rand::random::<f32>() * std::f32::consts::TAU;

                // Alpha based on damage (more damage = more opaque)
                let base_alpha = if *is_kill { 0.8 } else { 0.5 };
                let alpha = (base_alpha
                    * config.intensity
                    * damage_alpha_scale
                    * profile_alpha_multiplier(*profile))
                    .clamp(0.3, 1.0);

                let blood_texture = pick_spatter_texture(&atlas);

                // Create/update decal material
                let material = ForwardDecalMaterial {
                    base: StandardMaterial {
                        base_color_texture: blood_texture,
                        base_color: config.blood_color.with_alpha(alpha),
                        alpha_mode: AlphaMode::Blend,
                        cull_mode: None,
                        ..default()
                    },
                    extension: ForwardDecalMaterialExt {
                        depth_fade_factor: config.decal_depth_fade_factor.max(2.0),
                    },
                };

                if let Some(reuse_entity) = runtime.spatter_pool.pop() {
                    if let Ok((_, _, material_handle)) = query_spatters.get(reuse_entity) {
                        if let Some(existing_material) = decal_materials.get_mut(&material_handle.0) {
                            *existing_material = material;
                        }

                        commands.entity(reuse_entity).insert((
                            Name::new(format!("BloodSpatter_Reused_{}", i)),
                            Visibility::Visible,
                            BloodSpatter {
                                lifetime: config.spatter_lifetime,
                                total_lifetime: config.spatter_lifetime,
                                alpha,
                                base_alpha: alpha,
                                size,
                                wet_color: config.blood_color,
                                dry_color: config.dry_blood_color,
                                active: true,
                            },
                            build_spatter_transform(spatter_pos, base_normal, size, rotation),
                        ));

                        diagnostics.pooled_spatters_reused =
                            diagnostics.pooled_spatters_reused.saturating_add(1);
                    } else {
                        commands.spawn((
                            Name::new(format!("BloodSpatter_{}", i)),
                            ForwardDecal,
                            MeshMaterial3d(decal_materials.add(material)),
                            BloodSpatter {
                                lifetime: config.spatter_lifetime,
                                total_lifetime: config.spatter_lifetime,
                                alpha,
                                base_alpha: alpha,
                                size,
                                wet_color: config.blood_color,
                                dry_color: config.dry_blood_color,
                                active: true,
                            },
                            build_spatter_transform(spatter_pos, base_normal, size, rotation),
                        ));
                    }
                } else {
                    commands.spawn((
                        Name::new(format!("BloodSpatter_{}", i)),
                        ForwardDecal,
                        MeshMaterial3d(decal_materials.add(material)),
                        BloodSpatter {
                            lifetime: config.spatter_lifetime,
                            total_lifetime: config.spatter_lifetime,
                            alpha,
                            base_alpha: alpha,
                            size,
                            wet_color: config.blood_color,
                            dry_color: config.dry_blood_color,
                            active: true,
                        },
                        build_spatter_transform(spatter_pos, base_normal, size, rotation),
                    ));
                }
                active_spatter_count = active_spatter_count.saturating_add(1);

                diagnostics.active_spatters_spawned =
                    diagnostics.active_spatters_spawned.saturating_add(1);

                if config.enable_layered_effects {
                    // Placeholder layered counters to track profile distribution until
                    // dedicated mist/droplet render entities are added.
                    diagnostics.mist_spawned = diagnostics.mist_spawned.saturating_add(1);
                    diagnostics.droplets_spawned = diagnostics.droplets_spawned.saturating_add(1);
                }

                frame_spawn_budget = frame_spawn_budget.saturating_sub(1);
            }
        }
    }
}

/// System that fades out blood spatters over time and removes expired ones.
///
/// Spatters begin fading in the last 5 seconds of their lifetime.
pub fn blood_spatter_fade_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut BloodSpatter,
            &MeshMaterial3d<ForwardDecalMaterial<StandardMaterial>>,
        ),
        With<ForwardDecal>,
    >,
    time: Res<Time>,
    config: Res<BloodEffectConfig>,
    mut decal_materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
    mut runtime: ResMut<BloodEffectRuntime>,
    mut diagnostics: ResMut<BloodEffectDiagnostics>,
) {
    if !config.enable_blood {
        return;
    }

    let delta = time.delta_secs();
    for (entity, mut spatter, material_handle) in query.iter_mut() {
        if !spatter.active {
            continue;
        }

        spatter.lifetime -= delta;

        if spatter.lifetime <= 0.0 {
            spatter.active = false;
            diagnostics.pooled_spatters_returned =
                diagnostics.pooled_spatters_returned.saturating_add(1);
            commands.entity(entity).insert(Visibility::Hidden);
            runtime.spatter_pool.push(entity);
            continue;
        }

        let life_ratio = (spatter.lifetime / spatter.total_lifetime.max(0.001)).clamp(0.0, 1.0);
        let fade_start = config.fade_start_fraction.clamp(0.0, 0.95);
        let fade_t = if life_ratio <= fade_start {
            (life_ratio / fade_start.max(0.001)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        spatter.alpha = (spatter.base_alpha * fade_t).clamp(0.0, 1.0);

        let dryness = 1.0 - life_ratio;
        let wet = spatter.wet_color.to_srgba();
        let dry = spatter.dry_color.to_srgba();
        let color = Color::srgba(
            wet.red + (dry.red - wet.red) * dryness,
            wet.green + (dry.green - wet.green) * dryness,
            wet.blue + (dry.blue - wet.blue) * dryness,
            spatter.alpha,
        );

        if let Some(material) = decal_materials.get_mut(&material_handle.0) {
            material.base.base_color = color;
        }
    }

    if config.enable_diagnostics {
        diagnostics.accum_time_secs += delta;
        if diagnostics.accum_time_secs >= 5.0 {
            diagnostics.accum_time_secs = 0.0;
            log::info!(
                "[BloodDiagnostics] events={} spawned={} returned={} layered_mist={} layered_droplets={}",
                diagnostics.spatter_events,
                diagnostics.active_spatters_spawned,
                diagnostics.pooled_spatters_returned,
                diagnostics.mist_spawned,
                diagnostics.droplets_spawned
            );
        }
    }
}

/// Creates a procedural blood texture.
///
/// This generates a realistic blood splatter texture with:
/// - Dark red to bright red color variation
/// - Random spots and irregularities
/// - Organic splatter appearance with tendrils
fn create_blood_texture_variant(images: &mut Assets<Image>, variant_seed: usize) -> Handle<Image> {
    let texture_size = 96u32;
    let center = texture_size as f32 * 0.5;
    let seed = variant_seed as f32 + 1.0;

    let mut data = vec![0u8; (texture_size * texture_size * 4) as usize];

    let spot_count = 10 + (variant_seed % 6);
    let mut spots = Vec::with_capacity(spot_count);
    for i in 0..spot_count {
        let fi = i as f32;
        let angle = hash01(fi, seed, 3.1) * std::f32::consts::TAU;
        let radius = (0.18 + hash01(fi, seed, 4.7) * 0.55) * center;
        let x = center + radius * angle.cos();
        let y = center + radius * angle.sin();
        let size = 2.8 + hash01(fi, seed, 6.3) * 8.5;
        spots.push((x, y, size));
    }

    for y in 0..texture_size {
        for x in 0..texture_size {
            let x = x as f32;
            let y = y as f32;

            let dx = x - center;
            let dy = y - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let normalized = dist / center.max(1.0);

            let angle = dy.atan2(dx);
            let edge_noise = (angle * (4.0 + seed * 0.6)).sin() * 0.12
                + (angle * (8.5 + seed * 0.45)).cos() * 0.07
                + (hash01(x * 0.17, y * 0.19, seed) - 0.5) * 0.1;
            let irregular = normalized + edge_noise;

            let main_alpha = if irregular < 0.97 {
                let t = (irregular / 0.97).clamp(0.0, 1.0);
                (1.0 - t).powf(0.35)
            } else {
                0.0
            };

            let mut spot_alpha: f32 = 0.0;
            for (sx, sy, sr) in &spots {
                let sdx = x - *sx;
                let sdy = y - *sy;
                let sdist = (sdx * sdx + sdy * sdy).sqrt();
                if sdist < *sr {
                    let t = 1.0 - (sdist / *sr).clamp(0.0, 1.0);
                    spot_alpha = spot_alpha.max(t * t);
                }
            }

            let drip_mask = (((angle * 3.2 + seed).sin() * 0.5 + 0.5)
                * (1.0 - normalized).clamp(0.0, 1.0))
                * 0.2;

            let final_alpha = (main_alpha * 0.92 + spot_alpha * 0.85 + drip_mask).clamp(0.0, 1.0);
            if final_alpha < 0.03 {
                continue;
            }

            let color_noise = hash01(x * 0.31, y * 0.27, seed * 1.7);
            let wetness = (1.0 - normalized * 0.8 + color_noise * 0.18).clamp(0.0, 1.0);

            let base_r = 90.0 + wetness * 110.0;
            let base_g = 2.0 + wetness * 22.0;
            let base_b = 2.0 + wetness * 16.0;

            let dark_spot = if spot_alpha > 0.58 { 0.18 } else { 0.0 };
            let final_r = (base_r * (1.0 - dark_spot)).clamp(0.0, 255.0);
            let final_g = (base_g * (1.0 - dark_spot)).clamp(0.0, 255.0);
            let final_b = (base_b * (1.0 - dark_spot)).clamp(0.0, 255.0);

            let idx = ((y as u32 * texture_size + x as u32) * 4) as usize;
            data[idx] = final_r as u8;
            data[idx + 1] = final_g as u8;
            data[idx + 2] = final_b as u8;
            data[idx + 3] = (final_alpha * 255.0) as u8;
        }
    }

    let image = Image::new(
        Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    images.add(image)
}

/// Plugin that registers all blood spatter systems.
pub struct BloodSpatterPlugin;

impl Plugin for BloodSpatterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, initialize_blood_decal_atlas_system);
        app.add_systems(
            PostUpdate,
            (
                blood_spatter_on_death_system,
                blood_spatter_spawn_system,
                blood_spatter_fade_system,
            )
                .chain(),
        );
    }
}

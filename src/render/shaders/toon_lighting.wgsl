#define_import_path rose::toon_lighting

// Cel ("toon") lighting for the ROSE object and effect materials, used in place of
// Bevy's apply_pbr_lighting (shader def ROSE_TOON, see render/cel_shading.rs).
//
// Only directional lights are evaluated: the game spawns no point, spot or rect lights.
// - Key lights (sun, moon): N.L and the shadow map go through the hard bands in
//   toon_bands.wgsl.
// - Fill lights (the sky fill light, marked by affects_lightmapped_mesh_diffuse = false,
//   which only matters for Bevy lightmaps, unused here): a flat lift with no
//   terminator, so the key light's terminator is the only band edge across a model.
// - Ambient and environment-map light: flat, sampled with a normal bent toward the sky.
// Units, exposure and the atmosphere's sun tint match apply_pbr_lighting, so the result
// drops into the existing fog, tonemapping and auto-exposure chain.

#import bevy_pbr::{
    pbr_types::PbrInput,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    lighting,
    lighting::{LightingInput, LAYER_BASE},
    shadows,
    ambient,
    clustered_forward as clustering,
}
#import bevy_render::maths::PI
#import rose::toon_bands::{toon_step, toon_key_band, toon_shadow_band, TOON_EDGE_SOFTNESS}

#ifdef ENVIRONMENT_MAP
#import bevy_pbr::environment_map
#endif

#ifdef ATMOSPHERE
#import bevy_pbr::atmosphere::functions::{calculate_visible_sun_ratio, clamp_to_surface}
#endif

// Share of a fill light's radiance applied everywhere (PBR averages ~0.5 of it).
const TOON_FILL_LEVEL: f32 = 0.5;
// How far the ambient / environment normal is bent toward straight up
// (1 = one flat ambient color for every surface).
const TOON_INDIRECT_FLATTEN: f32 = 0.7;
// Rim light on the lit side of silhouettes: starts where 1 - N.V passes TOON_RIM_EDGE.
const TOON_RIM_EDGE: f32 = 0.72;
const TOON_RIM_STRENGTH: f32 = 0.3;
// Hard specular highlight (materials with a ROSE specular map only): N.H threshold
// and strength relative to the diffuse key light.
const TOON_SPECULAR_EDGE: f32 = 0.97;
const TOON_SPECULAR_STRENGTH: f32 = 0.6;

// `specular_strength` (0..1) scales the hard highlight; 0 disables it.
fn toon_lighting(in: PbrInput, specular_strength: f32) -> vec4<f32> {
    let base_color = in.material.base_color;
    let diffuse_color = base_color.rgb * (1.0 - in.material.metallic);
    let N = in.N;
    let V = in.V;
    let n_dot_v = max(dot(N, V), 0.0001);

    let view_z = dot(vec4<f32>(
        view_bindings::view.view_from_world[0].z,
        view_bindings::view.view_from_world[1].z,
        view_bindings::view.view_from_world[2].z,
        view_bindings::view.view_from_world[3].z
    ), in.world_position);

    // Radiance reaching the surface from key lights, already banded.
    var key_light = vec3<f32>(0.0);
    var fill_light = vec3<f32>(0.0);
    var specular_light = vec3<f32>(0.0);

    let n_directional_lights = view_bindings::lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
        let light = &view_bindings::lights.directional_lights[i];
        let L = (*light).direction_to_light;
        var radiance = (*light).color.rgb;

#ifdef ATMOSPHERE
        // Sun / moon color through the atmosphere, as in lighting::directional_light.
        let P_as = (view_bindings::atmosphere.world_to_atmosphere
            * vec4(in.world_position.xyz, 1.0)).xyz;
        let P_clamped = clamp_to_surface(view_bindings::atmosphere, P_as);
        let r = length(P_clamped);
        let mu_light = dot(L, normalize(P_clamped));
        radiance *= lighting::sample_transmittance_lut(r, mu_light)
            * calculate_visible_sun_ratio(
                view_bindings::atmosphere, r, mu_light, (*light).sun_disk_angular_size);
#endif

        let is_fill = ((*light).flags
            & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_AFFECTS_LIGHTMAPPED_MESH_DIFFUSE_BIT) == 0u;
        if (is_fill) {
            fill_light += radiance * TOON_FILL_LEVEL;
            continue;
        }

        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && ((*light).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_directional_shadow(
                i, in.world_position, in.world_normal, view_z, in.frag_coord.xy);
        }

        let band = min(toon_key_band(dot(N, L)), toon_shadow_band(shadow));
        key_light += radiance * band;

        if (specular_strength > 0.0) {
            let H = normalize(L + V);
            let highlight = toon_step(TOON_SPECULAR_EDGE, TOON_EDGE_SOFTNESS * 0.5, dot(N, H));
            specular_light += radiance * highlight * band * specular_strength * TOON_SPECULAR_STRENGTH;
        }
    }

    let rim = toon_step(TOON_RIM_EDGE, TOON_EDGE_SOFTNESS * 2.0, 1.0 - n_dot_v) * TOON_RIM_STRENGTH;
    let direct_light = (diffuse_color * (key_light * (1.0 + rim) + fill_light) + specular_light) / PI;

    // Flat indirect light: ambient plus the environment map's diffuse part, both seen
    // through a normal bent toward the sky so they do not shade across the model.
    let indirect_N = normalize(mix(N, vec3<f32>(0.0, 1.0, 0.0), TOON_INDIRECT_FLATTEN));
    var indirect_light = ambient::ambient_light(
        in.world_position, indirect_N, V, 1.0, diffuse_color, vec3<f32>(0.0), 1.0, vec3<f32>(1.0));

#ifdef ENVIRONMENT_MAP
    let cluster_index = clustering::view_fragment_cluster_index(
        in.frag_coord.xy, view_z, in.is_orthographic);
    var clusterable_object_index_ranges =
        clustering::unpack_clusterable_object_index_ranges(cluster_index);

    // Diffuse only: F0 = 0 zeroes the specular and multiscatter terms.
    var environment_input: LightingInput;
    environment_input.layers[LAYER_BASE].N = indirect_N;
    environment_input.layers[LAYER_BASE].R = indirect_N;
    environment_input.layers[LAYER_BASE].NdotV = 1.0;
    environment_input.layers[LAYER_BASE].perceptual_roughness = 1.0;
    environment_input.layers[LAYER_BASE].roughness = 1.0;
    environment_input.P = in.world_position.xyz;
    environment_input.V = V;
    environment_input.diffuse_color = diffuse_color;
    environment_input.metallic = 0.0;
    environment_input.F0_dielectric = vec3<f32>(0.0);
    environment_input.F0_metallic = vec3<f32>(0.0);
    environment_input.F_ab = vec2<f32>(0.0);
    let environment_light = environment_map::environment_map_light(
        &environment_input,
        &clusterable_object_index_ranges,
        false,
    );
    indirect_light += environment_light.diffuse;
#endif

    let emissive = in.material.emissive;
    let emissive_light = emissive.rgb * base_color.a
        * mix(1.0, view_bindings::view.exposure, emissive.a);

    return vec4<f32>(
        view_bindings::view.exposure * (direct_light + indirect_light) + emissive_light,
        base_color.a
    );
}

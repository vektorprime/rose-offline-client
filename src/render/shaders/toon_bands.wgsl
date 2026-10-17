#define_import_path rose::toon_bands

// Band shape shared by the cel-shaded ("toon") materials: the ROSE object and effect
// materials (toon_lighting.wgsl) and the terrain (terrain_material.wgsl).
// The values come from one shared read-only storage buffer written from
// CelShadingSettings (render/cel_shading.rs, Settings > Cel Shading), so they change
// live without re-preparing any material.

// Matches ToonParams in render/cel_shading.rs.
struct ToonParams {
    // x = N.L where the key light starts (terminator), y = N.L where it is fully on,
    // z = brightness of the middle band, w = half width of a band edge (N.L units)
    bands: vec4<f32>,
    // x = share of the key light kept in the shade band,
    // y = share of the ambient / environment light kept in the shade band,
    // z = shadow-map visibility counted as shadowed below this, w = shadow edge half width
    shade: vec4<f32>,
    // x = rim light strength, y = specular highlight strength,
    // z = 1 for cel lighting, 0 for Bevy PBR lighting, w = unused
    extras: vec4<f32>,
}

fn toon_step(edge: f32, softness: f32, x: f32) -> f32 {
    return smoothstep(edge - softness, edge + softness, x);
}

fn toon_enabled(params: ToonParams) -> bool {
    return params.extras.z > 0.5;
}

// N.L of a key light as three flat bands: 0 (shade), the mid level, 1 (lit).
fn toon_key_band(n_dot_l: f32, params: ToonParams) -> f32 {
    let mid_level = params.bands.z;
    return mid_level * toon_step(params.bands.x, params.bands.w, n_dot_l)
        + (1.0 - mid_level) * toon_step(params.bands.y, params.bands.w, n_dot_l);
}

// Filtered shadow-map visibility (0 = shadowed, 1 = lit) as a hard 0/1 edge.
fn toon_shadow_band(shadow: f32, params: ToonParams) -> f32 {
    return toon_step(params.shade.z, params.shade.w, shadow);
}

// Key light reaching a surface in a band: the shade band keeps `shade.x` of it.
fn toon_key_light_factor(band: f32, params: ToonParams) -> f32 {
    return mix(params.shade.x, 1.0, band);
}

// Ambient light reaching a surface whose key-light band is `band`.
fn toon_ambient_factor(band: f32, params: ToonParams) -> f32 {
    return mix(params.shade.y, 1.0, band);
}

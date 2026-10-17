#define_import_path rose::toon_bands

// Band shape shared by the cel-shaded ("toon") materials: the ROSE object and effect
// materials (toon_lighting.wgsl) and the terrain (terrain_material.wgsl).
// Tune the look here.

// N.L where the key light starts (the terminator) and where it is fully on.
const TOON_TERMINATOR: f32 = 0.0;
const TOON_FULL_LIGHT: f32 = 0.45;
// Brightness of the middle band, between the shade (0) and the lit band (1).
const TOON_MID_LEVEL: f32 = 0.55;
// Half width of a band edge, in N.L units: narrow enough to read as a hard edge,
// wide enough to avoid stair-stepping along the terminator.
const TOON_EDGE_SOFTNESS: f32 = 0.02;

// Shadow-map visibility below which a surface counts as shadowed, and the half width
// of that edge. Thresholding the filtered shadow gives crisp shadow outlines.
const TOON_SHADOW_THRESHOLD: f32 = 0.5;
const TOON_SHADOW_SOFTNESS: f32 = 0.08;

fn toon_step(edge: f32, softness: f32, x: f32) -> f32 {
    return smoothstep(edge - softness, edge + softness, x);
}

// N.L of a key light as three flat bands: 0 (shade), TOON_MID_LEVEL, 1 (lit).
fn toon_key_band(n_dot_l: f32) -> f32 {
    return TOON_MID_LEVEL * toon_step(TOON_TERMINATOR, TOON_EDGE_SOFTNESS, n_dot_l)
        + (1.0 - TOON_MID_LEVEL) * toon_step(TOON_FULL_LIGHT, TOON_EDGE_SOFTNESS, n_dot_l);
}

// Filtered shadow-map visibility (0 = shadowed, 1 = lit) as a hard 0/1 edge.
fn toon_shadow_band(shadow: f32) -> f32 {
    return toon_step(TOON_SHADOW_THRESHOLD, TOON_SHADOW_SOFTNESS, shadow);
}

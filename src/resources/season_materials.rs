use bevy::{pbr::StandardMaterial, prelude::*};

/// Pre-created materials and meshes for season particles to avoid ResMut conflicts
#[derive(Resource)]
pub struct SeasonMaterials {
    /// Leaf mesh for fall (diamond shape)
    pub leaf_mesh: Handle<Mesh>,
    /// Leaf materials for fall (orange, gold, brown colors)
    pub leaf_materials: Vec<Handle<StandardMaterial>>,
    /// Rain drop mesh for spring (elongated capsule)
    pub rain_mesh: Handle<Mesh>,
    /// Rain drop material for spring
    pub rain_material: Handle<StandardMaterial>,
    /// Flower mesh for spring (circle)
    pub flower_mesh: Handle<Mesh>,
    /// Flower materials for spring (pink, purple, yellow, white)
    pub flower_materials: Vec<Handle<StandardMaterial>>,
    /// Snow mesh for winter (hexagon for snowflake)
    pub snow_mesh: Handle<Mesh>,
    /// Snow material for winter
    pub snow_material: Handle<StandardMaterial>,
}

/// Setup system to create season materials once at startup
pub fn setup_season_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Create leaf materials for fall season
    let leaf_colors = vec![
        Color::srgb(0.8, 0.3, 0.1), // Orange-red
        Color::srgb(0.9, 0.5, 0.0), // Orange
        Color::srgb(0.8, 0.6, 0.1), // Gold
        Color::srgb(0.6, 0.2, 0.0), // Brown
    ];

    let leaf_materials: Vec<_> = leaf_colors
        .into_iter()
        .map(|color| {
            materials.add(StandardMaterial {
                base_color: color.with_alpha(0.9),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .collect();

    // Create rain material for spring
    let rain_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.8, 0.9, 0.7),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // Create flower materials for spring
    let flower_colors = vec![
        Color::srgb(1.0, 0.5, 0.8), // Pink
        Color::srgb(0.8, 0.5, 1.0), // Purple
        Color::srgb(1.0, 1.0, 0.5), // Yellow
        Color::srgb(1.0, 1.0, 1.0), // White
    ];

    let flower_materials: Vec<_> = flower_colors
        .into_iter()
        .map(|color| {
            materials.add(StandardMaterial {
                base_color: color,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .collect();

    // Create snow material for winter
    let snow_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.9),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // Create custom meshes for each particle type
    // Leaf mesh: Rhombus (diamond shape) for pointed leaf appearance (not circular)
    let leaf_mesh = meshes.add(Mesh::from(Rhombus::new(0.4, 0.8)));

    // Rain mesh: Elongated rectangle for rain drops (thin and tall)
    let rain_mesh = meshes.add(Mesh::from(Rectangle::new(0.1, 0.6)));

    // Flower mesh: Circle for flower petals
    let flower_mesh = meshes.add(Mesh::from(Circle::new(0.15)));

    // Snow mesh: Hexagon for snowflake appearance
    let snow_mesh = meshes.add(Mesh::from(RegularPolygon::new(0.25, 6)));

    commands.insert_resource(SeasonMaterials {
        leaf_mesh,
        leaf_materials,
        rain_mesh,
        rain_material,
        flower_mesh,
        flower_materials,
        snow_mesh,
        snow_material,
    });
}

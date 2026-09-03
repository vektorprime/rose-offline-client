use super::*;

pub(super) fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    block_x: u32,
    block_y: u32,
    water_size: f32,
    plane_start: Vec3,
    plane_end: Vec3,
    water_material: &Handle<WaterMaterial>, // Use custom WaterMaterial
) -> (Entity, Vec3, Vec2) {
    let start = Vec3::new(
        plane_start.x / 100.0,
        plane_start.y / 100.0,
        -plane_start.z / 100.0,
    );
    let end = Vec3::new(
        plane_end.x / 100.0,
        plane_end.y / 100.0,
        -plane_end.z / 100.0,
    );
    let uv_x = (end.x - start.x) / (water_size / 100.0);
    let uv_y = (end.z - start.z) / (water_size / 100.0);

    // Calculate water center and half extents for fish spawning
    let water_center = (start + end) * 0.5;
    let water_half_extents =
        Vec2::new((end.x - start.x).abs() * 0.5, (end.z - start.z).abs() * 0.5);

    let vertices = [
        ([start.x, start.y, end.z], [0.0, 1.0, 0.0], [uv_x, uv_y]),
        ([start.x, start.y, start.z], [0.0, 1.0, 0.0], [uv_x, 0.0]),
        ([end.x, start.y, start.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([end.x, start.y, end.z], [0.0, 1.0, 0.0], [0.0, uv_y]),
    ];
    let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);
    let collider_indices = vec![[0, 2, 1], [0, 3, 2]];

    let mut collider_verts = Vec::new();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (position, normal, uv) in &vertices {
        collider_verts.push((*position).into());
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_indices(indices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // Tight local-space bounds for this water quad. The mesh is built in world
    // XZ at its final height, while the entity transform is identity, so the
    // AABB can be derived directly from the plane corners (both Y ends for slopes).
    let water_min = Vec3::new(
        start.x.min(end.x),
        start.y.min(end.y) - 0.5,
        start.z.min(end.z),
    );
    let water_max = Vec3::new(
        start.x.max(end.x),
        start.y.max(end.y) + 0.5,
        start.z.max(end.z),
    );
    let water_aabb = Aabb::from_min_max(water_min, water_max);

    // Split spawn to avoid Bundle tuple limit (15+ components not supported)
    let water_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::Water,
            MapEditorWaterPlane::new(block_x, block_y, plane_start, plane_end, water_size),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(water_material.clone()),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            water_aabb,
            // Water lives on layer 1 so the reflection camera (layer 0) never
            // renders water into its own reflection.
            RenderLayers::layer(1),
            NotShadowCaster,
        ))
        .insert((
            NotShadowReceiver,
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices)
                .expect("Failed to create water collider"),
            CollisionGroups::new(COLLISION_GROUP_ZONE_WATER, COLLISION_FILTER_INSPECTABLE),
        ))
        .id();

    (water_entity, water_center, water_half_extents)
}

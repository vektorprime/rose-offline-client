//! Utility for projecting world-space positions onto mesh UV coordinates.
//!
//! This is used for placing blood stains and other effects accurately on skinned character models.

use bevy::prelude::*;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_mesh::{Indices, VertexAttributeValues};
// Mesh and Mesh3d are in the prelude

/// Result of a UV projection.
pub struct ProjectionResult {
    pub material_index: usize,
    pub uv: Vec2,
}

/// Projects a world-space position onto the UV coordinates of a skinned mesh.
///
/// # Arguments
/// * `world_pos` - The hit position in world space.
/// * `entity` - The entity with the `CharacterModel` component.
/// * `meshes` - Bevy mesh assets.
/// * `inverse_bindposes` - Bevy skinned mesh inverse bind pose assets.
/// * `transforms` - Query to get global transforms of joints and the model.
/// * `mesh_query` - Query to get Mesh3d components.
/// * `skinned_mesh_query` - Query to get SkinnedMesh components.
/// * `character_model` - The character model component containing mesh parts.
pub fn project_world_to_uv(
    world_pos: Vec3,
    entity: Entity,
    meshes: &Assets<Mesh>,
    inverse_bindposes: &Assets<SkinnedMeshInverseBindposes>,
    transforms: &Query<&GlobalTransform>,
    mesh_query: &Query<&Mesh3d>,
    skinned_mesh_query: &Query<&SkinnedMesh>,
    character_model: &crate::components::CharacterModel,
) -> Option<ProjectionResult> {
    let mut closest_dist = f32::MAX;
    let mut closest_uv = Vec2::ZERO;
    let mut closest_material_idx = 0;

    // Get the skinned mesh for the character if it exists
    let skinned_mesh = skinned_mesh_query.get(entity).ok();
    let inv_bind_poses = skinned_mesh.as_ref().and_then(|sm| {
        inverse_bindposes.get(&sm.inverse_bindposes)
    });

    for (part_enum, (_, part_entities)) in character_model.model_parts.iter() {
        let material_idx = part_enum as usize;

        for &mesh_entity in part_entities {
            let Ok(mesh_3d) = mesh_query.get(mesh_entity) else { continue };
            let Some(mesh) = meshes.get(&mesh_3d.0) else { continue };

            let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                Some(VertexAttributeValues::Float32x3(pos)) => pos,
                _ => continue,
            };
            let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                Some(VertexAttributeValues::Float32x2(uv)) => uv,
                _ => continue,
            };
            let indices = match mesh.indices() {
                Some(Indices::U32(idx)) => idx.clone(),
                Some(Indices::U16(idx)) => idx.iter().map(|&i| i as u32).collect(),
                _ => continue,
            };
        
            // Handle skinning
            let joint_indices = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
            let joint_weights = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
        
            let mesh_global_transform = transforms.get(mesh_entity).ok()?.to_matrix();

            // We iterate through triangles to find the closest point
            for chunk in indices.chunks(3) {
                if chunk.len() < 3 { continue; }
                let i0 = chunk[0] as usize;
                let i1 = chunk[1] as usize;
                let i2 = chunk[2] as usize;

                let p0_local = Vec3::from(positions[i0]);
                let p1_local = Vec3::from(positions[i1]);
                let p2_local = Vec3::from(positions[i2]);

                let uv0 = Vec2::from(uvs[i0]);
                let uv1 = Vec2::from(uvs[i1]);
                let uv2 = Vec2::from(uvs[i2]);

                let p0_world = transform_vertex(p0_local, i0, skinned_mesh, inv_bind_poses, transforms, &mesh_global_transform, joint_indices, joint_weights);
                let p1_world = transform_vertex(p1_local, i1, skinned_mesh, inv_bind_poses, transforms, &mesh_global_transform, joint_indices, joint_weights);
                let p2_world = transform_vertex(p2_local, i2, skinned_mesh, inv_bind_poses, transforms, &mesh_global_transform, joint_indices, joint_weights);

                if let Some((dist, uv)) = closest_point_on_triangle(world_pos, p0_world, p1_world, p2_world, uv0, uv1, uv2) {
                    if dist < closest_dist {
                        closest_dist = dist;
                        closest_uv = uv;
                        closest_material_idx = material_idx;
                    }
                }
            }
        }
    }

    if closest_dist < f32::MAX {
        Some(ProjectionResult {
            material_index: closest_material_idx,
            uv: closest_uv,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closest_point_on_triangle() {
        let p = Vec3::new(0.1, 0.1, 0.0);
        let a = Vec3::ZERO;
        let b = Vec3::X;
        let c = Vec3::Y;
        let uv_a = Vec2::ZERO;
        let uv_b = Vec2::X;
        let uv_c = Vec2::Y;

        let result = closest_point_on_triangle(p, a, b, c, uv_a, uv_b, uv_c);
        assert!(result.is_some());
        let (dist, uv) = result.unwrap();
        assert!(dist < 0.1);
        assert!(uv.x > 0.0 && uv.x < 1.0);
        assert!(uv.y > 0.0 && uv.y < 1.0);
    }
}

fn transform_vertex(
    local_pos: Vec3,
    vertex_idx: usize,
    skinned_mesh: Option<&SkinnedMesh>,
    inv_bind_poses: Option<&SkinnedMeshInverseBindposes>,
    transforms: &Query<&GlobalTransform>,
    mesh_global_transform: &Mat4,
    joint_indices_attr: Option<&VertexAttributeValues>,
    joint_weights_attr: Option<&VertexAttributeValues>,
) -> Vec3 {
    if let (Some(sm), Some(ibp), Some(indices_attr), Some(weights_attr)) = (skinned_mesh, inv_bind_poses, joint_indices_attr, joint_weights_attr) {
        let v_indices = match indices_attr {
            VertexAttributeValues::Uint16x4(idx) => idx[vertex_idx],
            VertexAttributeValues::Uint32x4(idx) => idx[vertex_idx].map(|v| v as u16),
            _ => return (*mesh_global_transform * Vec4::from((local_pos, 1.0))).xyz(),
        };
        let v_weights = match weights_attr {
            VertexAttributeValues::Float32x4(w) => w[vertex_idx],
            _ => return (*mesh_global_transform * Vec4::from((local_pos, 1.0))).xyz(),
        };

        let mut world_pos = Vec3::ZERO;
        for i in 0..4 {
            let joint_idx = v_indices[i] as usize;
            let weight = v_weights[i];
            if weight <= 0.0 { continue; }

            if let Some(&joint_entity) = sm.joints.get(joint_idx) {
                if let Ok(joint_transform) = transforms.get(joint_entity) {
                    let joint_global = joint_transform.to_matrix();
                    let inv_bind: Mat4 = ibp[joint_idx];
                    world_pos += weight * (joint_global * inv_bind * Vec4::from((local_pos, 1.0))).xyz();
                }
            }
        }
        world_pos
    } else {
        (*mesh_global_transform * Vec4::from((local_pos, 1.0))).xyz()
    }
}

fn closest_point_on_triangle(
    p: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    uv_a: Vec2,
    uv_b: Vec2,
    uv_c: Vec2,
) -> Option<(f32, Vec2)> {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return Some((p.distance(a), uv_a));
    }

    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return Some((p.distance(b), uv_b));
    }

    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return Some((p.distance(c), uv_c));
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        let pos = a + v * ab;
        return Some((p.distance(pos), uv_a + v * (uv_b - uv_a)));
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        let pos = a + w * ac;
        return Some((p.distance(pos), uv_a + w * (uv_c - uv_a)));
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let pos = b + w * (c - b);
        return Some((p.distance(pos), uv_b + w * (uv_c - uv_b)));
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let pos = a + v * ab + w * ac;
    let uv = uv_a + v * (uv_b - uv_a) + w * (uv_c - uv_a);
    Some((p.distance(pos), uv))
}

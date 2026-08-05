use std::{
    collections::HashMap,
    ffi::OsString,
    future::Future,
    path::{Path, PathBuf},
};

use bevy::asset::RenderAssetUsages;
use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext},
    ecs::component::Component,
    math::Vec3,
    prelude::Mesh,
    reflect::TypePath,
};
use bevy_mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use rose_file_readers::{RoseFile, ZmsFile};

/// Enables load-time normal smoothing. Flip to false and rebuild to A/B compare.
pub const SMOOTH_NORMALS: bool = true;
/// Faces whose normal deviates more than this from the vertex reference normal
/// are excluded, preserving intentional hard edges.
pub const SMOOTH_NORMALS_CREASE_ANGLE: f32 = 60.0_f32.to_radians();
/// Position tolerance (world units) for welding vertices during smoothing.
pub const SMOOTH_NORMALS_WELD_EPSILON: f32 = 0.0001;

use crate::render::{MESH_ATTRIBUTE_UV_1, MESH_ATTRIBUTE_UV_2, MESH_ATTRIBUTE_UV_3};

#[derive(Debug, TypePath, Clone, Asset)]
pub struct ZmsMaterialNumFaces {
    pub material_num_faces: Vec<u16>,
}

#[derive(Component, Clone)]
pub struct ZmsMaterialNumFacesHandle(pub bevy::prelude::Handle<ZmsMaterialNumFaces>);

#[derive(Default, TypePath)]
pub struct ZmsAssetLoader;

#[derive(Default, TypePath)]
pub struct ZmsNoSkinAssetLoader;

/// Converts a ZMS file into a Bevy mesh.
/// When `skip_joints` is set, joint data is not loaded. This is critical for
/// preventing bind group layout mismatches with effect meshes, which use the
/// non-skinned pipeline (model_only_mesh_bind_group) instead of the skinned
/// pipeline (skinned_mesh_layout).
async fn load_zms_mesh(
    reader: &mut dyn Reader,
    load_context: &mut LoadContext<'_>,
    skip_joints: bool,
) -> Result<Mesh, anyhow::Error> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;

    match <ZmsFile as RoseFile>::read((&bytes).into(), &Default::default()) {
        Ok(mut zms) => {
            if !zms.normal.is_empty() {
                for vert in zms.normal.iter_mut() {
                    let y = vert[1];
                    vert[1] = vert[2];
                    vert[2] = -y;
                }
            }

            if !zms.position.is_empty() {
                for vert in zms.position.iter_mut() {
                    let y = vert[1];
                    vert[1] = vert[2];
                    vert[2] = -y;
                }
            }

            if SMOOTH_NORMALS {
                zms.normal = smooth_normals(&zms.position, &zms.indices, &zms.normal);
            }

            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            mesh.insert_indices(Indices::U32(zms.indices));

            if !zms.normal.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, zms.normal);
            } else {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    vec![[0.0, 1.0, 0.0]; zms.position.len()],
                );
            }

            if !zms.position.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, zms.position);
            }

            if !zms.tangent.is_empty() {
                for vert in zms.tangent.iter_mut() {
                    let y = vert[1];
                    vert[1] = vert[2];
                    vert[2] = -y;
                }
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, zms.tangent);
            }

            if !zms.color.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, zms.color);
            }

            if !skip_joints {
                if !zms.bone_weights.is_empty() {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, zms.bone_weights);
                }

                if !zms.bone_indices.is_empty() {
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_JOINT_INDEX,
                        VertexAttributeValues::Uint16x4(zms.bone_indices),
                    );
                }
            }

            if !zms.uv1.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, zms.uv1);
            }

            if !zms.uv2.is_empty() {
                mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, zms.uv2);
            }

            if !zms.uv3.is_empty() {
                mesh.insert_attribute(MESH_ATTRIBUTE_UV_2, zms.uv3);
            }

            if !zms.uv4.is_empty() {
                mesh.insert_attribute(MESH_ATTRIBUTE_UV_3, zms.uv4);
            }

            if !zms.material_num_faces.is_empty() {
                load_context.labeled_asset_scope("material_num_faces".to_string(), |_lc| {
                    Ok::<ZmsMaterialNumFaces, anyhow::Error>(ZmsMaterialNumFaces {
                        material_num_faces: zms.material_num_faces,
                    })
                });
            }

            Ok(mesh)
        }
        Err(error) => Err(error),
    }
}

fn corner_angle(a: Vec3, b: Vec3) -> f32 {
    let denom = (a.length_squared() * b.length_squared()).sqrt();
    if denom < 1e-12 {
        return 0.0;
    }
    (a.dot(b) / denom).clamp(-1.0, 1.0).acos()
}

fn angle_between_normalized(a: Vec3, b: Vec3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos()
}

/// Recomputes vertex normals so that vertices duplicated at the same position
/// (hard edges from the exporter's smoothing groups) shade consistently.
///
/// For every vertex, the angle-weighted average of the faces touching all
/// vertices welded to its position is accumulated, excluding faces whose
/// normal deviates from the vertex's own average normal by more than
/// [`SMOOTH_NORMALS_CREASE_ANGLE`] so intentional hard edges stay sharp.
fn smooth_normals(
    positions: &[[f32; 3]],
    indices: &[u32],
    stored_normals: &[[f32; 3]],
) -> Vec<[f32; 3]> {
    let vertex_count = positions.len();
    if vertex_count == 0 || indices.len() < 3 {
        return stored_normals.to_vec();
    }

    let face_count = indices.len() / 3;
    let mut face_normals = vec![Vec3::ZERO; face_count];
    let mut corner_angles = vec![[0.0_f32; 3]; face_count];

    for face in 0..face_count {
        let ia = indices[face * 3] as usize;
        let ib = indices[face * 3 + 1] as usize;
        let ic = indices[face * 3 + 2] as usize;
        if ia >= vertex_count || ib >= vertex_count || ic >= vertex_count {
            continue;
        }
        let pa = Vec3::from(positions[ia]);
        let pb = Vec3::from(positions[ib]);
        let pc = Vec3::from(positions[ic]);
        let ab = pb - pa;
        let ac = pc - pa;
        let normal = ab.cross(ac);
        let area2 = normal.length();
        if area2 < 1e-12 {
            continue;
        }
        face_normals[face] = normal / area2;
        corner_angles[face] = [
            corner_angle(ab, ac),
            corner_angle(-ab, pc - pb),
            corner_angle(-ac, pb - pc),
        ];
    }

    if !stored_normals.is_empty() {
        let mut agreement = 0.0_f32;
        for face in 0..face_count {
            if face_normals[face] == Vec3::ZERO {
                continue;
            }
            for corner in 0..3 {
                let v = indices[face * 3 + corner] as usize;
                if v < stored_normals.len() {
                    agreement += face_normals[face].dot(Vec3::from(stored_normals[v]));
                }
            }
        }
        if agreement < 0.0 {
            for normal in face_normals.iter_mut() {
                *normal = -*normal;
            }
        }
    }

    let mut vertex_faces: Vec<Vec<(usize, usize)>> = vec![Vec::new(); vertex_count];
    for face in 0..face_count {
        for corner in 0..3 {
            let v = indices[face * 3 + corner] as usize;
            if v < vertex_count {
                vertex_faces[v].push((face, corner));
            }
        }
    }

    let mut group_of_vertex = vec![0_u32; vertex_count];
    let mut groups: Vec<Vec<u32>> = Vec::new();
    let mut group_map: HashMap<[i32; 3], u32> = HashMap::new();
    let inv_eps = 1.0 / SMOOTH_NORMALS_WELD_EPSILON;
    for (i, p) in positions.iter().enumerate() {
        let key = [
            (p[0] * inv_eps).round() as i32,
            (p[1] * inv_eps).round() as i32,
            (p[2] * inv_eps).round() as i32,
        ];
        let group = *group_map.entry(key).or_insert_with(|| {
            groups.push(Vec::new());
            (groups.len() - 1) as u32
        });
        group_of_vertex[i] = group;
        groups[group as usize].push(i as u32);
    }

    let mut smoothed = vec![[0.0_f32; 3]; vertex_count];
    for v in 0..vertex_count {
        let mut reference = Vec3::ZERO;
        for &(face, corner) in &vertex_faces[v] {
            reference += face_normals[face] * corner_angles[face][corner];
        }
        let reference = match reference.try_normalize() {
            Some(normal) => normal,
            None => {
                smoothed[v] = if v < stored_normals.len() {
                    stored_normals[v]
                } else {
                    [0.0, 1.0, 0.0]
                };
                continue;
            }
        };

        let mut accum = Vec3::ZERO;
        for &u in &groups[group_of_vertex[v] as usize] {
            for &(face, corner) in &vertex_faces[u as usize] {
                let face_normal = face_normals[face];
                if face_normal == Vec3::ZERO {
                    continue;
                }
                if angle_between_normalized(face_normal, reference)
                    <= SMOOTH_NORMALS_CREASE_ANGLE
                {
                    accum += face_normal * corner_angles[face][corner];
                }
            }
        }
        smoothed[v] = accum.try_normalize().unwrap_or(reference).into();
    }

    smoothed
}

impl AssetLoader for ZmsAssetLoader {
    type Asset = Mesh;
    type Settings = ();
    type Error = anyhow::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move { load_zms_mesh(reader, load_context, false).await }
    }

    fn extensions(&self) -> &[&str] {
        &["zms", "ZMS"]
    }
}

impl ZmsNoSkinAssetLoader {
    pub fn convert_path(path: &Path) -> PathBuf {
        let mut os_string: OsString = path.into();
        os_string.push(".no_skin");
        os_string.into()
    }
}

impl AssetLoader for ZmsNoSkinAssetLoader {
    type Asset = Mesh;
    type Settings = ();
    type Error = anyhow::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move { load_zms_mesh(reader, load_context, true).await }
    }

    fn extensions(&self) -> &[&str] {
        &["no_skin"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooths_duplicated_vertices_of_flat_quad() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 6];
        let smoothed = smooth_normals(&positions, &[0, 1, 2, 3, 4, 5], &normals);
        for normal in &smoothed {
            assert!((normal[0]).abs() < 1e-5);
            assert!((normal[1]).abs() < 1e-5);
            assert!((normal[2] - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn preserves_hard_edge_beyond_crease_angle() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 0.0, -1.0],
        ];
        let mut normals = vec![[0.0, 0.0, 1.0]; 3];
        normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 3]);
        let smoothed = smooth_normals(&positions, &[0, 1, 2, 3, 4, 5], &normals);
        for i in 0..3 {
            let n = Vec3::from(smoothed[i]);
            assert!(n.dot(Vec3::Z) > 0.99, "face 1 vertex {i} normal: {n}");
        }
        for i in 3..6 {
            let n = Vec3::from(smoothed[i]);
            assert!(n.dot(Vec3::Y) > 0.99, "face 2 vertex {i} normal: {n}");
        }
    }

    #[test]
    fn smooths_curved_surface_across_welded_vertices() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, -s, s],
        ];
        let mut normals = vec![[0.0, -1.0, 0.0]; 3];
        normals.extend_from_slice(&[[0.0, -s, -s]; 3]);
        let smoothed = smooth_normals(&positions, &[0, 1, 2, 3, 4, 5], &normals);
        let shared_a = Vec3::from(smoothed[0]);
        let shared_b = Vec3::from(smoothed[3]);
        assert!(shared_a.abs_diff_eq(shared_b, 1e-5));
        let shared_c = Vec3::from(smoothed[1]);
        let shared_d = Vec3::from(smoothed[4]);
        assert!(shared_c.abs_diff_eq(shared_d, 1e-5));
        let expected_blend = (Vec3::NEG_Y + Vec3::new(0.0, -s, -s)).normalize();
        assert!(shared_a.dot(expected_blend) > 0.999);
        assert!(Vec3::from(smoothed[2]).abs_diff_eq(Vec3::NEG_Y, 1e-5));
        assert!(Vec3::from(smoothed[5]).abs_diff_eq(Vec3::new(0.0, -s, -s), 1e-5));
    }

    #[test]
    fn flips_winding_to_match_stored_normals() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 3];
        let smoothed = smooth_normals(&positions, &[0, 2, 1], &normals);
        for normal in &smoothed {
            assert!((normal[2] - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn keeps_fallback_for_unused_vertices() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [5.0, 5.0, 5.0],
        ];
        let mut normals = vec![[0.0, 0.0, 1.0]; 3];
        normals.push([0.0, 1.0, 0.0]);
        let smoothed = smooth_normals(&positions, &[0, 1, 2], &normals);
        assert_eq!(smoothed[3], [0.0, 1.0, 0.0]);
    }
}

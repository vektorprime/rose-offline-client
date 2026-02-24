use std::{
    ffi::OsString,
    future::Future,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext},
    ecs::component::Component,
    prelude::Mesh,
    reflect::TypePath,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
    tasks::futures_lite::AsyncReadExt,
};
use log::info;
use rose_file_readers::{RoseFile, ZmsFile};

// Memory tracking and asset lifecycle logging added for diagnostics
use crate::render::{MESH_ATTRIBUTE_UV_1, MESH_ATTRIBUTE_UV_2, MESH_ATTRIBUTE_UV_3};

#[derive(Debug, TypePath, Clone, Asset)]
pub struct ZmsMaterialNumFaces {
    pub material_num_faces: Vec<u16>,
}

#[derive(Component, Clone)]
pub struct ZmsMaterialNumFacesHandle(pub bevy::prelude::Handle<ZmsMaterialNumFaces>);

#[derive(Default)]
pub struct ZmsAssetLoader;

#[derive(Default)]
pub struct ZmsNoSkinAssetLoader;

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
        async move {
            let mut bytes = Vec::new();
            use bevy::tasks::futures_lite::AsyncReadExt;
            reader.read_to_end(&mut bytes).await?;
            
            let asset_path = load_context.path().to_string_lossy();

            match <ZmsFile as RoseFile>::read((&bytes).into(), &Default::default()) {
                Ok(mut zms) => {
                    let has_bone_weights = !zms.bone_weights.is_empty();
                    let has_bone_indices = !zms.bone_indices.is_empty();
                    let bone_weight_count = zms.bone_weights.len();
                    let bone_index_count = zms.bone_indices.len();
                    // log::error!(
                    //     "[ZMS LOADER] ========== LOAD CALLED =========="
                    // );
                    // log::error!(
                    //     "[ZMS LOADER] Loading mesh: {} - vertices={}, bone_weights={}, bone_indices={}",
                    //     asset_path,
                    //     zms.position.len(),
                    //     bone_weight_count,
                    //     bone_index_count
                    // );
                    
                    let mut mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                    );
                    mesh.insert_indices(Indices::U16(zms.indices));

                    if !zms.normal.is_empty() {
                        for vert in zms.normal.iter_mut() {
                            let y = vert[1];
                            vert[1] = vert[2];
                            vert[2] = -y;
                        }
                        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, zms.normal);
                    } else {
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_NORMAL,
                            vec![[0.0, 1.0, 0.0]; zms.position.len()],
                        );
                    }

                    if !zms.position.is_empty() {
                        for vert in zms.position.iter_mut() {
                            let y = vert[1];
                            vert[1] = vert[2];
                            vert[2] = -y;
                        }
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

                    if !zms.bone_weights.is_empty() {
                        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, zms.bone_weights);
                    }

                    if !zms.bone_indices.is_empty() {
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_JOINT_INDEX,
                            VertexAttributeValues::Uint16x4(zms.bone_indices),
                        );
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
                        load_context.labeled_asset_scope(
                            "material_num_faces".to_string(),
                            |_lc| ZmsMaterialNumFaces {
                                material_num_faces: zms.material_num_faces,
                            },
                        );
                    }

                    Ok(mesh)
                }
                Err(error) => Err(error),
            }
        }
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
        async move {
            let mut bytes = Vec::new();
            use bevy::tasks::futures_lite::AsyncReadExt;
            reader.read_to_end(&mut bytes).await?;

            match <ZmsFile as RoseFile>::read((&bytes).into(), &Default::default()) {
                Ok(mut zms) => {
                    let mut mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                    );
                    mesh.insert_indices(Indices::U16(zms.indices));

                    if !zms.normal.is_empty() {
                        for vert in zms.normal.iter_mut() {
                            let y = vert[1];
                            vert[1] = vert[2];
                            vert[2] = -y;
                        }
                        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, zms.normal);
                    } else {
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_NORMAL,
                            vec![[0.0, 1.0, 0.0]; zms.position.len()],
                        );
                    }

                    if !zms.position.is_empty() {
                        for vert in zms.position.iter_mut() {
                            let y = vert[1];
                            vert[1] = vert[2];
                            vert[2] = -y;
                        }
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

                    // NOTE: ZmsNoSkinAssetLoader intentionally does NOT load joint data
                    // This is critical for preventing bind group layout mismatches with effect meshes
                    // Effect meshes should use the non-skinned pipeline (model_only_mesh_bind_group)
                    // not the skinned pipeline (skinned_mesh_layout)

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
                        load_context.labeled_asset_scope(
                            "material_num_faces".to_string(),
                            |_lc| ZmsMaterialNumFaces {
                                material_num_faces: zms.material_num_faces,
                            },
                        );
                    }

                    Ok(mesh)
                }
                Err(error) => Err(error),
            }
        }
    }

    fn extensions(&self) -> &[&str] {
        &["no_skin"]
    }
}

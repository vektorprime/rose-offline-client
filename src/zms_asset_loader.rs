use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{io::Reader, Asset, AssetLoader, BoxedFuture, LoadContext},
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

#[derive(Default)]
pub struct ZmsAssetLoader;

#[derive(Default)]
pub struct ZmsNoSkinAssetLoader;

impl AssetLoader for ZmsAssetLoader {
    type Asset = Mesh;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a, 'b>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'b>,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            use bevy::tasks::futures_lite::AsyncReadExt;
            reader.read_to_end(&mut bytes).await?;
            
            let asset_path = load_context.path().to_string_lossy();
            //info!("[ASSET LIFECYCLE] Loading ZMS mesh asset: {}", asset_path);
            //info!("[ASSET LIFECYCLE] Asset size: {} bytes", bytes.len());

            match <ZmsFile as RoseFile>::read((&bytes).into(), &Default::default()) {
                Ok(mut zms) => {
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
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
        })
    }

    fn extensions(&self) -> &[&str] {
        &["zms"]
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

    fn load<'a, 'b>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'b>,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            use bevy::tasks::futures_lite::AsyncReadExt;
            reader.read_to_end(&mut bytes).await?;

            match <ZmsFile as RoseFile>::read((&bytes).into(), &Default::default()) {
                Ok(mut zms) => {
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
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
        })
    }

    fn extensions(&self) -> &[&str] {
        &["no_skin"]
    }
}

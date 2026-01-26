use bevy::{
    asset::{Asset, Handle, UntypedAssetId, UntypedHandle, load_internal_asset},
    ecs::{
        query::QueryItem,
        system::{
            SystemParamItem, lifetimeless::{Read, SRes}
        },
    },
    math::Vec2,
    pbr::{
        AlphaMode, MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        App, Component, FromWorld, Material, MaterialPlugin, Mesh, Plugin,
        Vec3, With, World,
    },
    reflect::Reflect,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            AsBindGroup, BindGroupLayout, BlendComponent, BlendFactor, BlendOperation, BlendState, CompareFunction, RenderPipelineDescriptor, Shader, ShaderDefVal, ShaderRef, SpecializedMeshPipelineError, encase::ShaderType
        },
        texture::Image,
    },
    utils::Uuid,
};

use rose_file_readers::{ZscMaterialBlend, ZscMaterialGlow};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    MESH_ATTRIBUTE_UV_1,
};

use std::any::TypeId;

pub const OBJECT_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid {
        type_id: TypeId::of::<Shader>(),
        uuid: Uuid::from_u128(0xb7ebbc00ea16d3c7),
    });

#[derive(Default)]
pub struct ObjectMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for ObjectMaterialPlugin {
    fn build(&self, app: &mut App) {
        // Load the internal asset using the Bevy 0.13 API
        load_internal_asset!(
            app,
            OBJECT_MATERIAL_SHADER_HANDLE.typed::<Shader>(),
            "shaders/object_material.wgsl",
            bevy::render::render_resource::Shader::from_wgsl
        );

        app.add_plugins(ExtractComponentPlugin::<ObjectMaterialClipFace>::extract_visible());

        app.register_type::<ObjectMaterial>();

        app.add_plugins(MaterialPlugin::<ObjectMaterial> {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });

        app.register_type::<ObjectMaterialClipFace>();
    }
}

#[derive(Copy, Clone, Component, Reflect)]
pub enum ObjectMaterialClipFace {
    First(u32),
    Last(u32),
}

impl ExtractComponent for ObjectMaterialClipFace {
    type QueryData = &'static Self;
    type QueryFilter = With<Handle<ObjectMaterial>>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(*item)
    }
}

pub struct DrawObjectMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawObjectMesh {
    type Param = SRes<RenderAssets<Mesh>>;
    type ViewQuery = ();
    type ItemQuery = (Read<Handle<Mesh>>, Option<Read<ObjectMaterialClipFace>>);

    #[inline]
    fn render<'w>(
        _: &P,
        _: bevy::ecs::query::ROQueryItem<'_, Self::ViewQuery>,
        item: Option<bevy::ecs::query::ROQueryItem<'_, Self::ItemQuery>>,
        meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (mesh_handle, clip_face) = match item {
            Some((mesh_handle, clip_face)) => (mesh_handle, clip_face),
            None => return RenderCommandResult::Failure,
        };
        let (start_index_offset, end_index_offset) = if let Some(clip_face) = clip_face {
            match clip_face {
                ObjectMaterialClipFace::First(num_faces) => (num_faces * 3, 0),
                ObjectMaterialClipFace::Last(num_faces) => (0, num_faces * 3),
            }
        } else {
            (0, 0)
        };

        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    let start_index = start_index_offset;
                    let end_index = *count - end_index_offset;
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(start_index..end_index, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed => {
                    let start_vertex = start_index_offset;
                    let end_vertex = gpu_mesh.vertex_count - end_index_offset;
                    pass.draw(start_vertex..end_vertex, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

type DrawObjectMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<ObjectMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawObjectMesh,
);

// NOTE: These must match the bit flags in shaders/object_material.wgsl!
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ObjectMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const ALPHA_MODE_BLEND           = (1 << 2);
        const HAS_ALPHA_VALUE            = (1 << 3);
        const SPECULAR                   = (1 << 4);
        const NONE                       = 0;
    }
}

#[derive(Clone, ShaderType)]
pub struct ObjectMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub alpha_value: f32,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

impl From<&ObjectMaterial> for ObjectMaterialUniformData {
    fn from(material: &ObjectMaterial) -> ObjectMaterialUniformData {
        let mut flags = ObjectMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        let mut alpha_value = 1.0;

        if material.specular_texture.is_some() {
            flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE | ObjectMaterialFlags::SPECULAR;
            alpha_cutoff = 1.0;
        } else {
            if material.alpha_enabled {
                flags |= ObjectMaterialFlags::ALPHA_MODE_BLEND;

                if let Some(alpha_ref) = material.alpha_test {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_MASK;
                    alpha_cutoff = alpha_ref;
                }
            } else {
                flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
            }

            if let Some(material_alpha_value) = material.alpha_value {
                if material_alpha_value == 1.0 {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
                } else {
                    flags |= ObjectMaterialFlags::HAS_ALPHA_VALUE;
                    alpha_value = material_alpha_value;
                }
            }
        }

        ObjectMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff,
            alpha_value,
            lightmap_uv_offset: material.lightmap_uv_offset,
            lightmap_uv_scale: material.lightmap_uv_scale,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Reflect)]
pub enum ObjectMaterialBlend {
    #[default]
    Normal,
    Lighten,
}

impl From<ZscMaterialBlend> for ObjectMaterialBlend {
    fn from(zsc: ZscMaterialBlend) -> Self {
        match zsc {
            ZscMaterialBlend::Normal => ObjectMaterialBlend::Normal,
            ZscMaterialBlend::Lighten => ObjectMaterialBlend::Lighten,
        }
    }
}

#[derive(Copy, Clone, Debug, Reflect)]
pub enum ObjectMaterialGlow {
    Simple(Vec3),
    Light(Vec3),
    Texture(Vec3),
    TextureLight(Vec3),
    Alpha(Vec3),
}

impl From<ZscMaterialGlow> for ObjectMaterialGlow {
    fn from(zsc: ZscMaterialGlow) -> Self {
        match zsc {
            ZscMaterialGlow::Simple(value) => {
                ObjectMaterialGlow::Simple(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Light(value) => {
                ObjectMaterialGlow::Light(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Texture(value) => {
                ObjectMaterialGlow::Texture(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::TextureLight(value) => {
                ObjectMaterialGlow::TextureLight(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Alpha(value) => {
                ObjectMaterialGlow::Alpha(Vec3::new(value.x, value.y, value.z))
            }
        }
    }
}

#[derive(Asset, Debug, Clone, Reflect, AsBindGroup)]
#[uniform(0, ObjectMaterialUniformData)]
#[bind_group_data(ObjectMaterialKey)]
pub struct ObjectMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,

    #[texture(3)]
    #[sampler(4)]
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,

    #[texture(5)]
    #[sampler(6)]
    pub specular_texture: Option<Handle<Image>>,

    pub alpha_value: Option<f32>,
    pub alpha_enabled: bool,
    pub alpha_test: Option<f32>,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub skinned: bool,
    pub blend: ObjectMaterialBlend,
    pub glow: Option<ObjectMaterialGlow>,
}

#[derive(Clone)]
pub struct ObjectMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for ObjectMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        ObjectMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

impl Material for ObjectMaterial {
    fn vertex_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> bevy::prelude::AlphaMode {
        let mut alpha_mode;

        if self.specular_texture.is_some() {
            alpha_mode = AlphaMode::Opaque;
        } else {
            if self.alpha_enabled {
                alpha_mode = AlphaMode::Blend;

                if let Some(alpha_ref) = self.alpha_test {
                    alpha_mode = AlphaMode::Mask(alpha_ref);
                }
            } else {
                alpha_mode = AlphaMode::Opaque;
            }

            if let Some(material_alpha_value) = self.alpha_value {
                if material_alpha_value == 1.0 {
                    alpha_mode = AlphaMode::Opaque;
                } else {
                    alpha_mode = AlphaMode::Blend;
                }
            }
        }

        alpha_mode
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = key.bind_group_data.z_write_enabled;

        if !key.bind_group_data.z_test_enabled {
            descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;
        }

        if key.bind_group_data.two_sided {
            descriptor.primitive.cull_mode = None;
        }

        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ];

        if key.bind_group_data.has_lightmap {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("VERTEX_UVS_LIGHTMAP".into(), true));

            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::Bool("VERTEX_UVS_LIGHTMAP".into(), true));
            }

            vertex_attributes.push(MESH_ATTRIBUTE_UV_1.at_shader_location(3));
        } else if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("ZONE_LIGHTING_CHARACTER".into(), true));
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
        }

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
        {
            return Ok(());
        }

        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }

        Ok(())
    }
}

impl Default for ObjectMaterial {
    fn default() -> Self {
        Self {
            base_texture: None,
            alpha_value: None,
            alpha_enabled: false,
            alpha_test: None,
            two_sided: false,
            z_test_enabled: true,
            z_write_enabled: true,
            specular_texture: None,
            skinned: false,
            blend: ObjectMaterialBlend::Normal,
            glow: None,
            lightmap_texture: None,
            lightmap_uv_offset: Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObjectMaterialKey {
    has_lightmap: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
}

impl From<&ObjectMaterial> for ObjectMaterialKey {
    fn from(material: &ObjectMaterial) -> Self {
        ObjectMaterialKey {
            has_lightmap: material.lightmap_texture.is_some(),
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
        }
    }
}

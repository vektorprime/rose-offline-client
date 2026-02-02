use bevy::{
    asset::{Asset, Handle, load_internal_asset},
    pbr::{MaterialPipeline, MeshPipelineKey},
    prelude::{App, Image, Material, MaterialPlugin, Mesh, Plugin, Assets, Res, ResMut, Query},
    reflect::TypePath,
    render::{
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{
            AsBindGroup, CompareFunction, RenderPipelineDescriptor, Shader, SpecializedMeshPipelineError
        },
    },
};

use crate::resources::{ZoneTime, ZoneTimeState};

pub const SKY_MATERIAL_SHADER_HANDLE_TYPED: Handle<Shader> =
    Handle::weak_from_u128(0xadc5cbbc7a53fe);

#[derive(Default)]
pub struct SkyMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for SkyMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SKY_MATERIAL_SHADER_HANDLE_TYPED,
            "shaders/sky_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(
            MaterialPlugin::<SkyMaterial> {
                prepass_enabled: self.prepass_enabled,
                ..Default::default()
            },
        );
        app.add_systems(bevy::prelude::Update, sky_material_system);
        bevy::log::info!("[MATERIAL PLUGIN] SkyMaterial plugin built");
    }
}

#[derive(Asset, Debug, Clone, TypePath, AsBindGroup)]
pub struct SkyMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture_day: Option<Handle<Image>>,

    #[texture(2)]
    #[sampler(3)]
    pub texture_night: Option<Handle<Image>>,

    #[uniform(4)]
    pub day_weight: f32,
}

impl Default for SkyMaterial {
    fn default() -> Self {
        Self {
            texture_day: None,
            texture_night: None,
            day_weight: 1.0,
        }
    }
}

impl Material for SkyMaterial {
    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE_TYPED.into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE_TYPED.into()
    }

    fn alpha_mode(&self) -> bevy::render::alpha::AlphaMode {
        bevy::render::alpha::AlphaMode::Opaque
    }

    fn depth_bias(&self) -> f32 {
        9999999999.0
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

        descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
        {
            return Ok(());
        }

        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

pub fn sky_material_system(
    zone_time: Res<ZoneTime>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    query: Query<&Handle<SkyMaterial>>,
) {
    let day_weight = match zone_time.state {
        ZoneTimeState::Morning => zone_time.state_percent_complete,
        ZoneTimeState::Day => 1.0f32,
        ZoneTimeState::Evening => 1.0f32 - zone_time.state_percent_complete,
        ZoneTimeState::Night => 0.0f32,
    };

    for handle in query.iter() {
        if let Some(material) = sky_materials.get_mut(handle) {
            if (material.day_weight - day_weight).abs() > 0.001 {
                material.day_weight = day_weight;
            }
        }
    }
}

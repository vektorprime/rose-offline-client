//! Material extension for object materials with lightmaps and specular maps
//!
//! This extension adds ROSE-specific features to Bevy's StandardMaterial:
//! - Lightmap support with UV offset and scale
//! - Specular map support
//! - UV-space blood overlay
//!
//! Note: Zone lighting has been temporarily removed to simplify the rendering
//! pipeline. It can be added back later once basic rendering is confirmed working.

use bevy::material::OpaqueRendererMethod;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    StandardMaterial,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy_image::Image;
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_shader::{ShaderDefVal, ShaderRef};

use crate::render::MESH_ATTRIBUTE_UV_1;

/// Material extension for ROSE object materials
///
/// Extends StandardMaterial with:
/// - Lightmap texture and parameters
/// - Specular map texture
/// - Blood overlay texture and parameters
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(RoseObjectExtensionKey)]
pub struct RoseObjectExtension {
    /// Lightmap parameters: x, y = the part's (column, row) cell in the lightmap page,
    /// z = scale (1 / parts per row), w = parts per row of a shared lightmap page whose
    /// cell comes from the part's `MeshTag` (see `rose_object_extension.wgsl`)
    #[uniform(100)]
    pub lightmap_params: Vec4,

    /// Lightmap texture
    #[texture(101)]
    #[sampler(102)]
    pub lightmap_texture: Option<Handle<Image>>,

    /// Specular map texture. Only set for ZSC materials with the specular flag;
    /// the shader applies it only then (`ROSE_OBJECT_SPECULAR`).
    #[texture(103)]
    #[sampler(104)]
    pub specular_texture: Option<Handle<Image>>,

    /// Blood overlay texture painted in UV space during combat.
    #[texture(106)]
    #[sampler(107)]
    pub blood_overlay_texture: Option<Handle<Image>>,

    /// Blood parameters:
    /// - x: overlay intensity [0..1]
    /// - y: enabled flag (0 disabled, 1 enabled)
    /// - z/w: reserved
    #[uniform(108)]
    pub blood_params: Vec4,
}

impl Default for RoseObjectExtension {
    fn default() -> Self {
        Self {
            lightmap_params: Vec4::new(0.0, 0.0, 1.0, 0.0),
            lightmap_texture: None,
            specular_texture: None,
            blood_overlay_texture: None,
            blood_params: Vec4::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

/// Pipeline key of [`RoseObjectExtension`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoseObjectExtensionKey {
    /// Without a lightmap the shader must not apply one: the unbound texture is
    /// Bevy's white fallback, which the 2x lightmap blend would turn into a brightening.
    has_lightmap: bool,
    /// Likewise the specular map: unbound, it would read as reflectance 1.0.
    has_specular: bool,
}

impl From<&RoseObjectExtension> for RoseObjectExtensionKey {
    fn from(extension: &RoseObjectExtension) -> Self {
        Self {
            has_lightmap: extension.lightmap_texture.is_some(),
            has_specular: extension.specular_texture.is_some(),
        }
    }
}

/// Builds a ROSE object material (zone object parts, model parts). Every
/// `ExtendedMaterial<StandardMaterial, RoseObjectExtension>` must be created through
/// this, so that it is always forward-rendered.
///
/// With `DefaultOpaqueRendererMethod::deferred()` an `Auto` material is drawn only
/// through the deferred G-buffer, which has no room for the lightmap, the specular
/// map or the blood overlay (only the forward branch of `rose_object_extension.wgsl`
/// applies them). Alpha-masked parts were shaded twice (G-buffer, then forward), and
/// the water reflection camera has no deferred prepass, so Bevy skipped opaque
/// deferred parts there entirely.
pub fn rose_object_material(
    base: StandardMaterial,
    extension: RoseObjectExtension,
) -> ExtendedMaterial<StandardMaterial, RoseObjectExtension> {
    ExtendedMaterial {
        base: StandardMaterial {
            opaque_render_method: OpaqueRendererMethod::Forward,
            ..base
        },
        extension,
    }
}

impl MaterialExtension for RoseObjectExtension {
    fn fragment_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_OBJECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_OBJECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // This also runs for the prepass pipelines (depth, normals, shadows), which
        // never sample the lightmap or specular map and have their own vertex layout
        // (uv_b at location 2).
        if descriptor
            .vertex
            .shader_defs
            .contains(&ShaderDefVal::from("PREPASS_PIPELINE"))
        {
            return Ok(());
        }

        if key.bind_group_data.has_specular {
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment.shader_defs.push("ROSE_OBJECT_SPECULAR".into());
            }
        }

        if !key.bind_group_data.has_lightmap {
            return Ok(());
        }

        // ZMS meshes carry the lightmap UVs (ZMS uv2) in the custom MESH_ATTRIBUTE_UV_1,
        // which Bevy's mesh pipeline does not bind. Feed it to the forward shaders as
        // uv_b (location 3, VERTEX_UVS_B), as the original object material did.
        if layout.0.contains(MESH_ATTRIBUTE_UV_1) && !layout.0.contains(Mesh::ATTRIBUTE_UV_1) {
            let lightmap_uv = layout
                .0
                .get_layout(&[MESH_ATTRIBUTE_UV_1.at_shader_location(3)])?;
            if let Some(vertex_buffer) = descriptor.vertex.buffers.first_mut() {
                vertex_buffer.attributes.extend(lightmap_uv.attributes);
                descriptor.vertex.shader_defs.push("VERTEX_UVS_B".into());
                if let Some(fragment) = descriptor.fragment.as_mut() {
                    fragment.shader_defs.push("VERTEX_UVS_B".into());
                }
            }
        }

        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push("ROSE_OBJECT_LIGHTMAP".into());
        }
        Ok(())
    }
}

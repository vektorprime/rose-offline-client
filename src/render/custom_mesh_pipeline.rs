//! Shared pipeline infrastructure for custom mesh materials.
//!
//! This module provides common components used by TerrainMaterial, ObjectMaterial,
//! and EffectMeshMaterial to reduce code duplication and establish consistent patterns
//! for custom mesh rendering pipelines.
//!
//! # Bind Group Layout Standardization
//! - Group 0: View (camera, projection) - standard Bevy
//! - Group 1: Mesh (transforms, skinning) - standard Bevy
//! - Group 2: Material-specific data (textures, uniforms)
//! - Group 3: Zone lighting (fog, ambient light, time of day)

use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::*, SystemParamItem},
    },
    pbr::{
        MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::{AlphaMode, Mesh},
    render::{
        mesh::{MeshVertexBufferLayout, MeshVertexAttribute},
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            BlendComponent, BlendFactor, BlendOperation, BlendState, CompareFunction,
            DepthStencilState, FragmentState, MultisampleState, PrimitiveState,
            RenderPipelineDescriptor, VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        view::ViewUniformOffset,
    },
};
use std::hash::{Hash, Hasher};

/// Wrapper for AlphaMode to make it Hash-compatible.
///
/// Since Bevy's AlphaMode doesn't implement Hash, we need to wrap it
/// for use in pipeline keys which require Hash.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HashableAlphaMode(pub AlphaMode);

impl Hash for HashableAlphaMode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash based on the discriminant and relevant values
        match self.0 {
            AlphaMode::Opaque => 0u8.hash(state),
            AlphaMode::Mask(cutoff) => {
                1u8.hash(state);
                cutoff.to_bits().hash(state);
            }
            AlphaMode::Blend => 2u8.hash(state),
            AlphaMode::Premultiplied => 3u8.hash(state),
            AlphaMode::Add => 4u8.hash(state),
            AlphaMode::Multiply => 5u8.hash(state),
        }
    }
}

impl HashableAlphaMode {
    /// Convert to Bevy's AlphaMode
    pub fn to_alpha_mode(&self) -> AlphaMode {
        self.0
    }
}

/// Common pipeline key structure for custom mesh materials.
///
/// This key captures the essential configuration parameters that affect
/// pipeline specialization across different material types.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CustomMeshPipelineKey {
    /// Whether the mesh is skinned (requires joint/weight attributes)
    pub skinned: bool,

    /// Whether the mesh has UVs for lightmapping (requires UV_1 attribute)
    pub vertex_uvs_lightmap: bool,

    /// Alpha mode for transparency handling (wrapped for Hash compatibility)
    pub alpha_mode: HashableAlphaMode,

    /// Whether to disable face culling (render both sides)
    pub two_sided: bool,
}

impl CustomMeshPipelineKey {
    /// Create a new pipeline key with default opaque settings
    pub fn new() -> Self {
        Self {
            skinned: false,
            vertex_uvs_lightmap: false,
            alpha_mode: HashableAlphaMode(AlphaMode::Opaque),
            two_sided: false,
        }
    }

    /// Set whether the mesh is skinned
    pub fn with_skinned(mut self, skinned: bool) -> Self {
        self.skinned = skinned;
        self
    }

    /// Set whether the mesh has lightmap UVs
    pub fn with_vertex_uvs_lightmap(mut self, has_lightmap: bool) -> Self {
        self.vertex_uvs_lightmap = has_lightmap;
        self
    }

    /// Set the alpha mode
    pub fn with_alpha_mode(mut self, alpha_mode: AlphaMode) -> Self {
        self.alpha_mode = HashableAlphaMode(alpha_mode);
        self
    }

    /// Set whether the mesh is two-sided
    pub fn with_two_sided(mut self, two_sided: bool) -> Self {
        self.two_sided = two_sided;
        self
    }
}

impl Default for CustomMeshPipelineKey {
    fn default() -> Self {
        Self::new()
    }
}

/// Compatibility conversion from TerrainMaterialPipelineKey
///
/// This allows the shared infrastructure to work with existing terrain materials
/// during the migration period.
impl From<crate::render::terrain_material::TerrainMaterialPipelineKey> for CustomMeshPipelineKey {
    fn from(_key: crate::render::terrain_material::TerrainMaterialPipelineKey) -> Self {
        // Terrain materials are typically opaque, non-skinned, and one-sided
        Self {
            skinned: false,
            vertex_uvs_lightmap: false,
            alpha_mode: HashableAlphaMode(AlphaMode::Opaque),
            two_sided: false,
        }
    }
}

/// Helper for building vertex buffer layouts for custom mesh materials.
///
/// This builder provides a consistent way to configure vertex attributes
/// across different material types, ensuring compatibility with shaders.
pub struct MeshVertexLayoutBuilder;

impl MeshVertexLayoutBuilder {
    /// Build vertex attributes list based on pipeline key configuration.
    ///
    /// This method creates a minimal set of attributes based on the pipeline key,
    /// excluding attributes that aren't needed for the current configuration.
    ///
    /// # Arguments
    /// * `key` - The pipeline key describing the mesh configuration
    ///
    /// # Returns
    /// A vector of `(MeshVertexAttribute, shader_location)` pairs.
    pub fn build_vertex_attributes(key: &CustomMeshPipelineKey) -> Vec<(MeshVertexAttribute, u32)> {
        let mut attributes = vec![
            (Mesh::ATTRIBUTE_POSITION, 0),
            (Mesh::ATTRIBUTE_NORMAL, 1),
            (Mesh::ATTRIBUTE_UV_0, 2),
        ];

        // Add lightmap UV if needed
        if key.vertex_uvs_lightmap {
            attributes.push((crate::render::MESH_ATTRIBUTE_UV_1, 3));
        }

        // Add skinning attributes if needed
        if key.skinned {
            attributes.push((Mesh::ATTRIBUTE_JOINT_INDEX, 4));
            attributes.push((Mesh::ATTRIBUTE_JOINT_WEIGHT, 5));
        }

        attributes
    }

    /// Get the vertex buffer layout for a given set of attributes.
    ///
    /// This method creates a `VertexBufferLayout` from a set of mesh vertex
    /// attributes, using the provided `MeshVertexBufferLayout` to determine
    /// the actual buffer layout.
    ///
    /// # Arguments
    /// * `layout` - The mesh vertex buffer layout
    /// * `attributes` - List of (attribute, shader_location) pairs
    ///
    /// # Returns
    /// A `VertexBufferLayout` compatible with the given attributes.
    pub fn get_layout(
        layout: &MeshVertexBufferLayout,
        attributes: &[(MeshVertexAttribute, u32)],
    ) -> Result<VertexBufferLayout, bevy::render::mesh::MissingVertexAttributeError> {
        layout.get_layout(
            &attributes
                .iter()
                .map(|(attr, loc)| attr.at_shader_location(*loc))
                .collect::<Vec<_>>(),
        )
    }
}

/// Helper functions for common pipeline descriptor setup.
///
/// These functions reduce code duplication by providing standard
/// configurations for depth/stencil state, blend state, and primitive state.
///
/// Note: Bevy's `SetMeshViewBindGroup` and `SetMeshBindGroup` are already
/// available from `bevy::pbr` and implement `RenderCommand` for use in
/// custom mesh pipelines.
pub struct PipelineDescriptorBuilder;

impl PipelineDescriptorBuilder {
    /// Create a standard depth stencil state for opaque rendering.
    ///
    /// # Returns
    /// A `DepthStencilState` configured for opaque rendering with depth testing and writing enabled.
    pub fn opaque_depth_stencil() -> DepthStencilState {
        DepthStencilState {
            format: bevy::render::render_resource::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Greater,
            stencil: bevy::render::render_resource::StencilState::default(),
            bias: bevy::render::render_resource::DepthBiasState::default(),
        }
    }

    /// Create a depth stencil state with custom configuration.
    ///
    /// # Arguments
    /// * `depth_write_enabled` - Whether to enable depth writing
    /// * `depth_compare` - The depth comparison function
    ///
    /// # Returns
    /// A `DepthStencilState` with the specified configuration.
    pub fn depth_stencil(
        depth_write_enabled: bool,
        depth_compare: CompareFunction,
    ) -> DepthStencilState {
        DepthStencilState {
            format: bevy::render::render_resource::TextureFormat::Depth32Float,
            depth_write_enabled,
            depth_compare,
            stencil: bevy::render::render_resource::StencilState::default(),
            bias: bevy::render::render_resource::DepthBiasState::default(),
        }
    }

    /// Create a standard blend state for alpha blending.
    ///
    /// # Returns
    /// A `BlendState` configured for standard alpha blending.
    pub fn alpha_blend_state() -> BlendState {
        BlendState {
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
        }
    }

    /// Create a blend state based on alpha mode.
    ///
    /// # Arguments
    /// * `alpha_mode` - The alpha mode to use
    ///
    /// # Returns
    /// A `BlendState` configured for the given alpha mode, or `None` for opaque.
    pub fn blend_state_from_alpha_mode(alpha_mode: AlphaMode) -> Option<BlendState> {
        match alpha_mode {
            AlphaMode::Opaque => None,
            AlphaMode::Mask(_) => None,
            AlphaMode::Blend => Some(Self::alpha_blend_state()),
            AlphaMode::Premultiplied => Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            }),
            AlphaMode::Add => Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            AlphaMode::Multiply => Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::Zero,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::Zero,
                    operation: BlendOperation::Add,
                },
            }),
        }
    }

    /// Create a standard primitive state for mesh rendering.
    ///
    /// # Arguments
    /// * `two_sided` - Whether to disable face culling
    ///
    /// # Returns
    /// A `PrimitiveState` configured for mesh rendering.
    pub fn primitive_state(two_sided: bool) -> PrimitiveState {
        PrimitiveState {
            topology: bevy::render::render_resource::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: bevy::render::render_resource::FrontFace::Ccw,
            cull_mode: if two_sided {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            unclipped_depth: false,
            polygon_mode: bevy::render::render_resource::PolygonMode::Fill,
            conservative: false,
        }
    }

    /// Create a standard multisample state.
    ///
    /// # Arguments
    /// * `msaa_samples` - The number of MSAA samples
    ///
    /// # Returns
    /// A `MultisampleState` configured with the specified MSAA samples.
    pub fn multisample_state(msaa_samples: u32) -> MultisampleState {
        MultisampleState {
            count: msaa_samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    /// Apply blend state to a fragment state based on alpha mode.
    ///
    /// This is a convenience method that modifies the fragment state in place.
    ///
    /// # Arguments
    /// * `fragment` - Mutable reference to the fragment state
    /// * `alpha_mode` - The alpha mode to apply
    pub fn apply_blend_state(
        fragment: &mut FragmentState,
        alpha_mode: HashableAlphaMode,
    ) {
        if let Some(blend_state) = Self::blend_state_from_alpha_mode(alpha_mode.to_alpha_mode()) {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(blend_state);
            }
        }
    }

    /// Create a standard pipeline descriptor from a mesh pipeline key.
    ///
    /// This method creates a base pipeline descriptor with standard configurations
    /// that can be further customized by material-specific specialization logic.
    ///
    /// # Arguments
    /// * `mesh_pipeline` - The base mesh pipeline
    /// * `mesh_key` - The mesh pipeline key
    /// * `custom_key` - The custom mesh pipeline key
    /// * `layout` - The mesh vertex buffer layout
    ///
    /// # Returns
    /// A `RenderPipelineDescriptor` with standard configurations applied.
    pub fn build_base_descriptor(
        mesh_pipeline: &MeshPipeline,
        mesh_key: MeshPipelineKey,
        custom_key: &CustomMeshPipelineKey,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, bevy::render::render_resource::SpecializedMeshPipelineError> {
        use bevy::render::render_resource::SpecializedMeshPipeline;
        let mut descriptor = mesh_pipeline.specialize(mesh_key, layout)?;

        // Apply custom key configurations
        descriptor.primitive = Self::primitive_state(custom_key.two_sided);

        // Apply alpha mode to fragment state
        if let Some(ref mut fragment) = descriptor.fragment {
            Self::apply_blend_state(fragment, custom_key.alpha_mode);
        }

        Ok(descriptor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_mesh_pipeline_key_default() {
        let key = CustomMeshPipelineKey::default();
        assert!(!key.skinned);
        assert!(!key.vertex_uvs_lightmap);
        assert_eq!(key.alpha_mode.to_alpha_mode(), AlphaMode::Opaque);
        assert!(!key.two_sided);
    }

    #[test]
    fn test_custom_mesh_pipeline_key_builder() {
        let key = CustomMeshPipelineKey::new()
            .with_skinned(true)
            .with_vertex_uvs_lightmap(true)
            .with_alpha_mode(AlphaMode::Blend)
            .with_two_sided(true);

        assert!(key.skinned);
        assert!(key.vertex_uvs_lightmap);
        assert_eq!(key.alpha_mode.to_alpha_mode(), AlphaMode::Blend);
        assert!(key.two_sided);
    }

    #[test]
    fn test_vertex_attributes() {
        let key = CustomMeshPipelineKey::new();
        let attrs = MeshVertexLayoutBuilder::build_vertex_attributes(&key);
        assert_eq!(attrs.len(), 3); // position, normal, uv_0

        let key = CustomMeshPipelineKey::new()
            .with_vertex_uvs_lightmap(true)
            .with_skinned(true);
        let attrs = MeshVertexLayoutBuilder::build_vertex_attributes(&key);
        assert_eq!(attrs.len(), 6); // + uv_1, joint_index, joint_weight
    }
}

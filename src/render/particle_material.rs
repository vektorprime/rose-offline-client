use bevy::{
    asset::{load_internal_asset, Handle},
    pbr::{Material, MaterialPlugin, MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::{
        alpha::AlphaMode,
        mesh::MeshVertexBufferLayoutRef,
        render_resource::*,
        storage::ShaderStorageBuffer,
    },
};

pub const PARTICLE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10002000);

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct ParticleMaterial {
    #[storage(0, read_only)]
    pub positions: Handle<ShaderStorageBuffer>,
    
    #[storage(1, read_only)]
    pub sizes: Handle<ShaderStorageBuffer>,
    
    #[storage(2, read_only)]
    pub colors: Handle<ShaderStorageBuffer>,
    
    #[storage(3, read_only)]
    pub textures: Handle<ShaderStorageBuffer>,
    
    #[texture(4)]
    #[sampler(5)]
    pub texture: Handle<Image>,
    
    #[uniform(6)]
    pub blend_op: u32,
    
    #[uniform(7)]
    pub src_blend_factor: u32,
    
    #[uniform(8)]
    pub dst_blend_factor: u32,
    
    #[uniform(9)]
    pub billboard_type: u32,
}

impl Material for ParticleMaterial {
    fn vertex_shader() -> ShaderRef {
        PARTICLE_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        PARTICLE_SHADER_HANDLE.into()
    }

    // Return Default to signal no custom prepass shader - this prevents Bevy from
    // processing prepass shaders that expect storage buffers when the prepass
    // pipeline only provides uniform buffers
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // DIAGNOSTIC: Log when specialize is called
        info!("[ParticleMaterial::specialize] Called for pipeline");
        info!("[ParticleMaterial::specialize] Pipeline label: {:?}", descriptor.label);
        
        // DIAGNOSTIC: Log bind group layouts count
        info!("[ParticleMaterial::specialize] Number of bind group layouts: {}", descriptor.layout.len());
        
        // DIAGNOSTIC: Log vertex buffers before modification
        info!("[ParticleMaterial::specialize] Vertex buffers BEFORE modification: {}", descriptor.vertex.buffers.len());
        for (i, vb) in descriptor.vertex.buffers.iter().enumerate() {
            info!("[ParticleMaterial::specialize]   Vertex buffer {}: stride={}, attrs={}", 
                i, vb.array_stride, vb.attributes.len());
        }
        
        // CRITICAL: Keep one empty vertex buffer layout (don't clear entirely)
        // This prevents index out of bounds errors in shadow/prepass systems
        descriptor.vertex.buffers = vec![VertexBufferLayout {
            array_stride: 0,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![],
        }];
        
        info!("[ParticleMaterial::specialize] Vertex buffers AFTER modification: {}", descriptor.vertex.buffers.len());
        
        // DIAGNOSTIC: Log fragment shader targets if present
        if let Some(fragment) = &descriptor.fragment {
            info!("[ParticleMaterial::specialize] Fragment targets: {}", fragment.targets.len());
        }
        
        info!("[ParticleMaterial::specialize] ✓ Specialization complete");
        
        Ok(())
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

pub struct ParticleMaterialPlugin;

impl Plugin for ParticleMaterialPlugin {
    fn build(&self, app: &mut App) {
        // Load main shader
        info!("[ParticleMaterial] Loading particle shader...");
        load_internal_asset!(
            app,
            PARTICLE_SHADER_HANDLE,
            "shaders/particle.wgsl",
            Shader::from_wgsl
        );
        
        // Register material plugin with prepass and shadow rendering disabled
        // Prepass is disabled because storage buffers cause pipeline validation issues
        // (Shader global ResourceBinding mismatch), and AlphaMode::Blend materials
        // are skipped in prepass anyway
        app.add_plugins(MaterialPlugin::<ParticleMaterial> {
            prepass_enabled: false,  // Disable prepass - storage buffers incompatible with prepass pipeline
            shadows_enabled: false,  // Transparent particles don't cast shadows
            ..Default::default()
        });
        
        // Add debug validation system (only in debug builds)
        #[cfg(debug_assertions)]
        {
            app.add_systems(Update, validate_particle_materials
                .run_if(resource_exists::<Assets<ParticleMaterial>>)
            );
            app.add_systems(Update, log_particle_material_bind_groups
                .run_if(resource_exists::<Assets<ParticleMaterial>>)
            );
        }
        
        info!("✓ [ParticleMaterial] Plugin initialized successfully");
        
        // DIAGNOSTIC: Log the expected bind group layout for ParticleMaterial
        info!("[ParticleMaterial] Expected bind group layout (from AsBindGroup derive):");
        info!("[ParticleMaterial]   Binding 0: Storage(read_only) - positions");
        info!("[ParticleMaterial]   Binding 1: Storage(read_only) - sizes");
        info!("[ParticleMaterial]   Binding 2: Storage(read_only) - colors");
        info!("[ParticleMaterial]   Binding 3: Storage(read_only) - textures");
        info!("[ParticleMaterial]   Binding 4: Texture - texture");
        info!("[ParticleMaterial]   Binding 5: Sampler - texture sampler");
        info!("[ParticleMaterial]   Binding 6: Uniform - blend_op");
        info!("[ParticleMaterial]   Binding 7: Uniform - src_blend_factor");
        info!("[ParticleMaterial]   Binding 8: Uniform - dst_blend_factor");
        info!("[ParticleMaterial]   Binding 9: Uniform - billboard_type");
    }
}

// Debug validation system - only runs in debug builds
#[cfg(debug_assertions)]
fn validate_particle_materials(
    materials: Res<Assets<ParticleMaterial>>,
    storage_buffers: Res<Assets<ShaderStorageBuffer>>,
    images: Res<Assets<Image>>,
    mut warned_materials: Local<std::collections::HashSet<AssetId<ParticleMaterial>>>,
) {
    for (id, material) in materials.iter() {
        // Skip if we've already warned about this material
        if warned_materials.contains(&id) {
            continue;
        }
        
        let mut has_error = false;
        
        // DIAGNOSTIC: Log material details
        info!("[ParticleMaterial] Validating material {:?}:", id);
        info!("[ParticleMaterial]   blend_op: {}", material.blend_op);
        info!("[ParticleMaterial]   src_blend_factor: {}", material.src_blend_factor);
        info!("[ParticleMaterial]   dst_blend_factor: {}", material.dst_blend_factor);
        info!("[ParticleMaterial]   billboard_type: {}", material.billboard_type);
        
        // Validate storage buffers
        if storage_buffers.get(&material.positions).is_none() {
            error!("⚠ [ParticleMaterial {:?}] Positions buffer not loaded!", id);
            error!("   Create with: storage_buffers.add(ShaderStorageBuffer::from(positions_data))");
            has_error = true;
        } else {
            info!("[ParticleMaterial]   ✓ Positions buffer loaded: {:?}", material.positions.id());
        }
        
        if storage_buffers.get(&material.sizes).is_none() {
            error!("⚠ [ParticleMaterial {:?}] Sizes buffer not loaded!", id);
            has_error = true;
        } else {
            info!("[ParticleMaterial]   ✓ Sizes buffer loaded: {:?}", material.sizes.id());
        }
        
        if storage_buffers.get(&material.colors).is_none() {
            error!("⚠ [ParticleMaterial {:?}] Colors buffer not loaded!", id);
            has_error = true;
        } else {
            info!("[ParticleMaterial]   ✓ Colors buffer loaded: {:?}", material.colors.id());
        }
        
        if storage_buffers.get(&material.textures).is_none() {
            error!("⚠ [ParticleMaterial {:?}] Textures buffer not loaded!", id);
            has_error = true;
        } else {
            info!("[ParticleMaterial]   ✓ Textures buffer loaded: {:?}", material.textures.id());
        }
        
        // Validate texture
        if images.get(&material.texture).is_none() {
            warn!("⚠ [ParticleMaterial {:?}] Texture not loaded yet", id);
            warn!("   This is normal during startup but may cause rendering issues");
        } else {
            info!("[ParticleMaterial]   ✓ Texture loaded: {:?}", material.texture.id());
        }
        
        if !has_error {
            debug!("✓ [ParticleMaterial {:?}] All assets validated", id);
        }
        
        // Mark as warned so we don't spam logs
        warned_materials.insert(id);
    }
}

/// DIAGNOSTIC: System to log bind group creation details for ParticleMaterial
/// This runs each frame to catch when new materials are added
#[cfg(debug_assertions)]
fn log_particle_material_bind_groups(
    materials: Res<Assets<ParticleMaterial>>,
    mut logged_materials: Local<std::collections::HashSet<AssetId<ParticleMaterial>>>,
) {
    for (id, _material) in materials.iter() {
        // Only log once per material
        if logged_materials.contains(&id) {
            continue;
        }
        
        info!("[ParticleMaterial] Material {:?} registered in Assets", id);
        info!("[ParticleMaterial] This material will use bind group with:");
        info!("[ParticleMaterial]   - Bindings 0-3: Storage buffers (read_only)");
        info!("[ParticleMaterial]   - Binding 4: Texture");
        info!("[ParticleMaterial]   - Binding 5: Sampler");
        info!("[ParticleMaterial]   - Bindings 6-9: Uniforms (u32 each)");
        
        logged_materials.insert(id);
    }
}

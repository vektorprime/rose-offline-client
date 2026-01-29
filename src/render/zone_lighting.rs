use bevy::{
    asset::{load_internal_asset, UntypedHandle, UntypedAssetId, Handle},
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    math::{Vec3, Vec4},
    pbr::CascadeShadowConfig,
    prelude::{
        AmbientLight, App, Color, Commands, DirectionalLight, DirectionalLightBundle, EulerRot,
        FromWorld, IntoSystemConfigs, Plugin, Quat, ReflectResource, Res, ResMut,
        Resource, Shader, Startup, Transform, World,
    },
    reflect::{Reflect, TypePath},
    render::{
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            encase, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferUsages, ShaderSize, ShaderStages,
            ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
    utils::Uuid,
};
use std::any::TypeId;

pub const ZONE_LIGHTING_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid { type_id: TypeId::of::<Shader>(), uuid: Uuid::from_u128(0x444949d32b35d5d9) });

pub const ZONE_LIGHTING_SHADER_HANDLE_TYPED: Handle<Shader> =
    Handle::weak_from_u128(0x444949d32b35d5d9);

fn default_light_transform() -> Transform {
    Transform::from_rotation(Quat::from_euler(
        EulerRot::ZYX,
        0.0,
        std::f32::consts::PI * (2.0 / 3.0),
        -std::f32::consts::PI / 4.0,
    ))
}

#[derive(Default)]
pub struct ZoneLightingPlugin;

impl Plugin for ZoneLightingPlugin {
    fn build(&self, app: &mut App) {
        bevy::log::info!("[ZONE LIGHTING] Building ZoneLightingPlugin");
        
        load_internal_asset!(
            app,
            ZONE_LIGHTING_SHADER_HANDLE_TYPED,
            "shaders/zone_lighting.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ZoneLighting>()
            .init_resource::<ZoneLighting>();

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            bevy::log::info!("[ZONE LIGHTING] Initializing render app systems");
            render_app
                .add_systems(ExtractSchedule, extract_uniform_data)
                .add_systems(Render, (prepare_uniform_data,).in_set(RenderSet::Prepare));
        } else {
            bevy::log::error!("[ZONE LIGHTING] FAILED to get render app - lighting will not work!");
        }

        app.add_systems(Startup, spawn_lights);
        bevy::log::info!("[ZONE LIGHTING] ZoneLightingPlugin build complete");
    }

    fn finish(&self, app: &mut App) {
        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };
        render_app.init_resource::<ZoneLightingUniformMeta>();
    }
}

fn spawn_lights(mut commands: Commands) {
    bevy::log::info!("[ZONE LIGHTING] Spawning directional and ambient lights");
    
    let light_entity = commands.spawn(DirectionalLightBundle {
        transform: default_light_transform(),
        directional_light: DirectionalLight {
            illuminance: 10000.0,  // Bevy 0.13 requires lux values (was ~1.0 in 0.12)
            shadows_enabled: true,
            ..Default::default()
        },
        cascade_shadow_config: CascadeShadowConfig {
            bounds: vec![10000.0],
            overlap_proportion: 2.0,
            minimum_distance: 0.1,
        },
        ..Default::default()
    }).id();
    
    bevy::log::info!("[ZONE LIGHTING] Directional light spawned: entity={:?}, illuminance=10000.0", light_entity);

    commands.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 80.0,  // Bevy 0.13 requires much higher values (was ~1.0 in 0.12)
    });
    
    bevy::log::info!("[ZONE LIGHTING] Ambient light inserted: brightness=80.0");
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ZoneLighting {
    pub map_ambient_color: Vec3,
    pub character_ambient_color: Vec3,
    pub character_diffuse_color: Vec3,
    pub light_direction: Vec3,

    pub color_fog_enabled: bool,
    pub fog_color: Vec3,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,

    pub alpha_fog_enabled: bool,
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,
    // Height-based fog parameters
    pub fog_min_height: f32,
    pub fog_max_height: f32,
    pub fog_height_density: f32,
    // Time of day parameters
    pub time_of_day: f32,
    pub day_color: Vec3,
    pub night_color: Vec3,
}

impl Default for ZoneLighting {
    fn default() -> Self {
        Self {
            map_ambient_color: Vec3::ONE,
            character_ambient_color: Vec3::ONE,
            character_diffuse_color: Vec3::ONE,
            light_direction: default_light_transform().back().normalize(),
            fog_color: Vec3::new(0.2, 0.2, 0.2),
            color_fog_enabled: true,
            fog_density: 0.0018,
            fog_min_density: 0.0,
            fog_max_density: 0.75,
            alpha_fog_enabled: true,
            fog_alpha_weight_start: 0.85,
            fog_alpha_weight_end: 0.98,
            // Height-based fog parameters
            fog_min_height: -10.0,
            fog_max_height: 50.0,
            fog_height_density: 0.5,
            // Time of day parameters
            time_of_day: 0.5, // 0.0 = night, 1.0 = day
            day_color: Vec3::new(0.7, 0.8, 1.0), // Day fog color (blueish)
            night_color: Vec3::new(0.1, 0.1, 0.3), // Night fog color (dark blue)
        }
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct ZoneLightingUniformData {
    pub map_ambient_color: Vec4,
    pub character_ambient_color: Vec4,
    pub character_diffuse_color: Vec4,
    pub light_direction: Vec4,

    pub fog_color: Vec4,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,
    
    // Height-based fog parameters
    pub fog_min_height: f32,
    pub fog_max_height: f32,
    pub fog_height_density: f32,
    
    // Time of day parameters
    pub time_of_day: f32,
    pub day_color: Vec4,
    pub night_color: Vec4,

    // TODO: Calculate camera far plane based on alpha fog:
    // far = sqrt(log2(1.0 - fog_alpha_weight_end) / (-fog_density * fog_density * 1.442695))
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,
}

#[derive(Resource)]
pub struct ZoneLightingUniformMeta {
    buffer: Buffer,
    bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for ZoneLightingUniformMeta {
    fn from_world(world: &mut World) -> Self {
        bevy::log::info!("[ZONE LIGHTING] Creating ZoneLightingUniformMeta render resources");
        
        let render_device = world.resource::<RenderDevice>();

        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: ZoneLightingUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("zone_lighting_uniform_buffer"),
        });
        bevy::log::info!("[ZONE LIGHTING] Uniform buffer created: size={} bytes",
            ZoneLightingUniformData::min_size().get());

        let bind_group_layout =
            render_device.create_bind_group_layout(
                Some("zone_lighting_uniform_layout"),
                &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ZoneLightingUniformData::min_size()),
                    },
                    count: None,
                }],
            );
        bevy::log::info!("[ZONE LIGHTING] Bind group layout created");

        let bind_group = render_device.create_bind_group(
            "zone_lighting_uniform_bind_group",
            &bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        );
        bevy::log::info!("[ZONE LIGHTING] Bind group created - ZoneLightingUniformMeta ready");

        ZoneLightingUniformMeta {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

fn extract_uniform_data(
    mut commands: Commands,
    zone_lighting: Extract<Res<ZoneLighting>>,
    mut frame_count: bevy::ecs::system::Local<u32>,
) {
    *frame_count += 1;
    
    // Log every 60 frames to avoid spam
    if *frame_count % 60 == 1 {
        bevy::log::info!("[ZONE LIGHTING] Extracting uniform data (frame {})", *frame_count);
        bevy::log::info!("[ZONE LIGHTING]   Map ambient: {:?}", zone_lighting.map_ambient_color);
        bevy::log::info!("[ZONE LIGHTING]   Light direction: {:?}", zone_lighting.light_direction);
        bevy::log::info!("[ZONE LIGHTING]   Fog enabled: {}, density: {}",
            zone_lighting.color_fog_enabled, zone_lighting.fog_density);
    }
    
    commands.insert_resource(ZoneLightingUniformData {
        map_ambient_color: zone_lighting.map_ambient_color.extend(1.0),
        character_ambient_color: zone_lighting.character_ambient_color.extend(1.0),
        character_diffuse_color: zone_lighting.character_diffuse_color.extend(1.0),
        light_direction: zone_lighting.light_direction.extend(1.0),
        fog_color: zone_lighting.fog_color.extend(1.0),
        fog_density: if zone_lighting.color_fog_enabled {
            zone_lighting.fog_density
        } else {
            0.0
        },
        fog_min_density: if zone_lighting.color_fog_enabled {
            zone_lighting.fog_min_density
        } else {
            0.0
        },
        fog_max_density: if zone_lighting.color_fog_enabled {
            zone_lighting.fog_max_density
        } else {
            0.0
        },
        // Height-based fog parameters
        fog_min_height: zone_lighting.fog_min_height,
        fog_max_height: zone_lighting.fog_max_height,
        fog_height_density: zone_lighting.fog_height_density,
        // Time of day parameters
        time_of_day: zone_lighting.time_of_day,
        day_color: zone_lighting.day_color.extend(1.0),
        night_color: zone_lighting.night_color.extend(1.0),
        fog_alpha_weight_start: if zone_lighting.alpha_fog_enabled {
            zone_lighting.fog_alpha_weight_start
        } else {
            99999999999.0
        },
        fog_alpha_weight_end: if zone_lighting.alpha_fog_enabled {
            zone_lighting.fog_alpha_weight_end
        } else {
            999999999.0
        },
    });
}

fn prepare_uniform_data(
    uniform_data: Res<ZoneLightingUniformData>,
    uniform_meta: ResMut<ZoneLightingUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; ZoneLightingUniformData::SHADER_SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&uniform_meta.buffer, 0, buffer.as_ref());
}

pub struct SetZoneLightingBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetZoneLightingBindGroup<I> {
    type Param = SRes<ZoneLightingUniformMeta>;
    type ItemQuery = ();
    type ViewQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, Self::ItemQuery>>,
        meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &meta.into_inner().bind_group, &[]);

        RenderCommandResult::Success
    }
}

#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
use animation::RoseAnimationPlugin;
use bevy::ecs::schedule::ApplyDeferred;
use bevy::{
    asset::AssetApp,
    camera::visibility::{Visibility, VisibilitySystems},
    camera::Camera,
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass},
    light::{Atmosphere, DirectionalLightShadowMap, EnvironmentMapLight, VolumetricFog},
    mesh::Mesh3d,
    pbr::{
        AtmosphereSettings, DefaultOpaqueRendererMethod, ExtendedMaterial, MaterialPlugin,
        MeshMaterial3d, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
        StandardMaterial,
    },
    post_process::{
        bloom::Bloom,
        dof::{DepthOfField, DepthOfFieldMode},
    },
    prelude::{
        default, in_state, resource_exists, App, AppExtStates, AssetServer, Assets, Camera3d,
        ClearColorConfig, Color, Commands, Entity, IntoScheduleConfigs, Msaa, OnEnter, OnExit,
        PerspectiveProjection, PluginGroup, PostStartup, PostUpdate, PreUpdate, Projection, Quat,
        Query, Res, ResMut, Startup, SystemSet, Transform, Update, Vec3, With, Without,
    },
    render::occlusion_culling::OcclusionCulling,
    render::view::ColorGrading,
    render::settings::{Backends, RenderCreation, WgpuFeatures, WgpuSettings},
    transform::{components::GlobalTransform, TransformSystems},
    window::{Window, WindowMode},
};
use bevy_egui::{egui, EguiContexts, PrimaryEguiContext};
use bevy_light::ShadowFilteringMethod;
use bevy_mesh::{Indices, Mesh, VertexAttributeValues};
use log::info;
// DISABLED: bevy_procedural_grass is not compatible with Bevy 0.18
// use bevy_procedural_grass::prelude::*;
use bevy_rapier3d::plugin::PhysicsSet;
use enum_map::enum_map;
use exe_resource_loader::{ExeResourceCursor, ExeResourceLoader};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
};

use rose_data::{CharacterMotionDatabaseOptions, NpcDatabaseOptions, ZoneId};
use rose_file_readers::{
    AruaVfsIndex, HostFilesystemDevice, IrosePhVfsIndex, LtbFile, StbFile, TitanVfsIndex, VfsIndex,
    VirtualFilesystem, VirtualFilesystemDevice, ZscFile,
};

pub mod animation;
pub mod audio;
pub mod bundles;
pub mod components;
pub mod debug;
pub mod effect_loader;
use effect_loader::EffectCache;
pub mod events;
pub mod exe_resource_loader;
pub mod graphics;
pub mod logging;
pub mod map_editor;
pub mod model_loader;
pub mod protocol;
pub mod render;
pub mod blood_effect_plugin;
pub mod dds_image_loader;
pub mod resources;
pub mod sailing;
pub mod scripting;
pub mod systems;
pub mod terrain;
pub mod ui;
pub mod vfs_asset_io;
pub mod zms_asset_loader;
pub mod zone_content;
pub mod zone_loader;

use audio::OddioPlugin;
use audio::{
    boat_loop_sound_update_system, boat_one_shot_sound_system, ensure_boat_sound_state_system,
    setup_boat_sound_assets,
};
use dds_image_loader::DdsImageLoader;
use events::{
    BankEvent, BoardBoatEvent, CharacterSelectEvent, ChatBubbleEvent, ChatboxEvent,
    ClanDialogEvent, ClientEntityEvent, ConversationDialogEvent, DisembarkBoatEvent,
    FlightToggleEvent, GameConnectionEvent, HitEvent, LoadZoneEvent, LoginEvent, MessageBoxEvent,
    MoveDestinationEffectEvent, MoveSpeedSetEvent, NetworkEvent, NpcStoreEvent,
    NumberInputDialogEvent, PartyEvent, PersonalStoreEvent, PingRequestEvent, PingResponseEvent,
    PingState, PlayerCommandEvent, QuestScrollEvent, QuestTriggerEvent, SpawnEffectEvent,
    SpawnProjectileEvent, SystemFuncEvent, UseItemEvent, WorldConnectionEvent, ZoneEvent,
    ZoneLoadedFromVfsEvent,
};
use model_loader::ModelLoader;
use render::{
    follow_sky_to_camera_system, moon_light_follow_camera_system, spawn_volumetric_clouds,
    toggle_atmosphere_based_on_time, update_starry_sky_night_factor, update_starry_sky_system,
    AtmosphereState,
    CameraUnderwaterState,
    ExtensionMaterialPlugin,
    MoonLight,
    ParticleMaterialPlugin,
    RoseEffectExtension,
    RoseObjectMaterialPlugin,
    RoseRenderPlugin,
    StarrySky,
    StarrySkyMaterial,
    StarrySkyMaterialPlugin,
    StarrySkySettings,
    UnderwaterEffectPlugin,
    // Old 2D cloud system (DISABLED):
    // CloudMaterialPlugin,
    // spawn_cloud_layer,
    // New 3D volumetric cloud system:
    VolumetricCloudPlugin,
    WaterMaterial,
    WaterReflectionPlugin,
    WorldUiRenderPlugin,
    ZoneLightingPlugin,
};
use resources::{
    load_ui_resources, run_network_thread, update_ui_resources,
    AppState, ClientEntityList, CurrentZone, DamageDigitsSpawner, DebugRenderConfig,
    FlightSettings, GameData, LoginCameraAnimation, MonsterChatterPhrases, NameTagSettings,
    NetworkThread, NetworkThreadMessage, RenderConfiguration, RenderExtractionDiagnostics,
    SelectedTarget, ServerConfiguration, SoundCache, SoundSettings, SpecularTexture, VfsResource,
    WaterSettings, WindSettings, WindState, WorldTime, ZoneTime,
};
use scripting::RoseScriptingPlugin;
use systems::{
    ability_values_system,
    add_monster_chatter_system,
    animation_effect_system,
    animation_sound_system,
    auto_login_system,
    background_music_system,
    boat_buoyancy_system,
    boat_toggle_system,
    boat_wake_spawn_system,
    boat_wake_update_system,
    character_model_add_collider_system,
    character_model_blink_system,
    character_model_update_system,
    character_select_enter_system,
    character_select_event_system,
    character_select_exit_system,
    character_select_input_system,
    character_select_models_system,
    character_select_system,
    chat_bubble_cleanup_system,
    chat_bubble_orphan_cleanup_system,
    chat_bubble_spawn_system,
    chat_bubble_update_system,
    clan_system,
    client_entity_event_system,
    collision_height_only_system,
    collision_player_system,
    collision_player_system_join_zone,
    command_system,
    conversation_dialog_system,
    cooldown_system,
    create_default_particle_texture,
    damage_number_animate_system,
    damage_number_billboard_system,
    directional_light_system,
    effect_system,
    ensure_boat_state_system,
    ensure_boat_wake_emitter_system,
    ensure_flight_state_system,
    facing_direction_system,
    flight_movement_system,
    flight_pose_blend_update_system,
    flight_pose_system,
    flight_toggle_system,
    free_camera_system,
    game_connection_system,
    game_keyboard_input_system,
    game_mouse_input_system,
    game_state_enter_system,
    game_zone_change_system,
    hit_event_system,
    item_drop_model_add_collider_system,
    item_drop_model_system,
    login_connection_system,
    login_event_system,
    login_state_enter_system,
    login_state_exit_system,
    login_system,
    model_viewer_enter_system,
    model_viewer_exit_system,
    model_viewer_system,
    monster_chatter_system,
    move_destination_effect_system,
    move_speed_set_system,
    memory_diagnostics_system,
    name_tag_system,
    name_tag_update_color_system,
    name_tag_update_healthbar_system,
    name_tag_visibility_system,
    network_thread_system,
    npc_idle_sound_system,
    npc_model_add_collider_system,
    npc_model_update_system,
    orbit_camera_system,
    particle_sequence_system,
    particle_storage_buffer_update_system,
    passive_recovery_system,
    pending_damage_system,
    pending_skill_effect_system,
    personal_store_model_add_collider_system,
    personal_store_model_system,
    player_command_system,
    projectile_system,
    quest_trigger_system,
    remote_boat_sync_system,
    sail_animation_system,
    sail_camera_system,
    sailing_movement_system,
    setup_boat_wake_assets,
    spawn_effect_system,
    spawn_projectile_system,
    status_effect_system,
    sync_vegetation_wind_system,
    system_func_event_system,
    update_position_system,
    use_item_event_system,
    vehicle_model_system,
    vehicle_sound_system,
    visible_status_effects_system,
    wind_update_system,
    world_connection_system,
    world_time_system,
    world_ui_occlusion_system,
    zone_time_system,
    zone_viewer_enter_system,
    BirdPlugin,
    CharacterSelectInputState,
    // DISABLED: color_grading_time_of_day_system conflicts with Bevy 0.16 Atmosphere
    // color_grading_time_of_day_system,
    DebugInspectorPlugin,
    DirtDashPlugin,
    FishPlugin,
    WindEffectPlugin,
    WingSpawnPlugin,
};
use ui::{
    admin_menu_keyboard_system, load_dialog_sprites_system, ui_admin_menu_system, ui_bank_system,
    ui_character_create_system, ui_character_info_system, ui_character_select_name_tag_system,
    ui_character_select_system, ui_chatbox_system, ui_clan_system, ui_create_clan_system,
    ui_debug_camera_info_system, ui_debug_client_entity_list_system,
    ui_debug_command_viewer_system, ui_debug_dialog_list_system,
    ui_debug_effect_list_system, ui_debug_entity_inspector_system, ui_debug_item_list_system,
    ui_debug_menu_system, ui_debug_npc_list_system,
    ui_debug_render_system, ui_debug_skill_list_system, ui_debug_zone_lighting_system,
    ui_debug_zone_list_system, ui_debug_zone_time_system, ui_drag_and_drop_system,
    ui_game_menu_system, ui_hotbar_system, ui_inventory_system, ui_item_drop_name_system,
    ui_login_system, ui_message_box_system, ui_minimap_system, ui_npc_store_system,
    ui_number_input_dialog_system, ui_party_option_system, ui_party_system,
    ui_personal_store_system, ui_player_info_system, ui_quest_list_system, ui_respawn_system,
    ui_sailing_hud_system, ui_selected_target_system, ui_server_select_system, ui_settings_system,
    ui_skill_list_system, ui_skill_tree_system, ui_sound_event_system, ui_status_effects_system,
    ui_window_sound_system, widgets::Dialog, DepthOfFieldSettings, DialogLoader, UiSoundEvent,
    UiStateAdminMenu, UiStateDebugWindows, UiStateDragAndDrop, UiStateWindows,
};
use vfs_asset_io::VfsAssetReaderPlugin;
use zms_asset_loader::{ZmsAssetLoader, ZmsMaterialNumFaces, ZmsNoSkinAssetLoader};
use zone_loader::{
    force_zone_visibility_system, zone_loaded_from_vfs_system, zone_loader_system,
    MemoryTrackingResource, ZoneLoadChannelReceiver, ZoneLoadChannelSender, ZoneLoaderAsset,
};

use crate::components::{SoundCategory, VegetationSwayPlugin};

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct AccountConfig {
    pub username: String,
    pub password: String,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct AutoLoginConfig {
    pub enabled: bool,
    pub channel_id: Option<usize>,
    pub server_id: Option<usize>,
    pub character_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "path")]
pub enum FilesystemDeviceConfig {
    #[serde(rename = "vfs")]
    Vfs(String),
    #[serde(rename = "directory")]
    Directory(String),
    #[serde(rename = "aruavfs")]
    AruaVfs(String),
    #[serde(rename = "titanvfs")]
    TitanVfs(String),
    #[serde(rename = "iroseph")]
    IrosePh(String),
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct FilesystemConfig {
    pub devices: Vec<FilesystemDeviceConfig>,
}

fn vfs_index_root_path(path: &str) -> PathBuf {
    // Get the parent directory of the VFS index file
    // For relative paths like "data.idx", parent() returns empty string
    // In that case, use the current directory
    Path::new(path)
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_default()
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

fn add_vfs_device(
    vfs_devices: &mut Vec<Box<dyn VirtualFilesystemDevice + Send + Sync>>,
    base_path: &mut Option<PathBuf>,
    device: Box<dyn VirtualFilesystemDevice + Send + Sync>,
    index_root_path: PathBuf,
    log_name: &str,
) {
    vfs_devices.push(device);
    log::info!(
        "Loading game data from {} root path {}",
        log_name,
        index_root_path.to_string_lossy()
    );
    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path.clone())));
    // Use the VFS root path as base path for saving
    if base_path.is_none() {
        *base_path = Some(index_root_path);
    }
}

impl FilesystemConfig {
    /// Creates a virtual filesystem from the configured devices.
    /// Returns a tuple of (VFS, base_path) where base_path is the real filesystem
    /// path where game data is stored. This is used for saving files back to disk.
    pub fn create_virtual_filesystem(&self) -> Option<(Arc<VirtualFilesystem>, PathBuf)> {
        let mut vfs_devices: Vec<Box<dyn VirtualFilesystemDevice + Send + Sync>> = Vec::new();
        let mut base_path: Option<PathBuf> = None;

        for device_config in self.devices.iter() {
            match device_config {
                FilesystemDeviceConfig::Directory(path) => {
                    log::info!("Loading game data from host directory {}", path);
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(path.into())));
                    // For directory-based VFS, the base path is the directory itself
                    // Only set if path is non-empty
                    if !path.is_empty() {
                        base_path = Some(PathBuf::from(path));
                    }
                }
                FilesystemDeviceConfig::AruaVfs(path) => {
                    let index_root_path = vfs_index_root_path(path);
                    log::info!("Loading game data from AruaVfs {}", path);
                    add_vfs_device(
                        &mut vfs_devices,
                        &mut base_path,
                        Box::new(
                            AruaVfsIndex::load(Path::new(path), &index_root_path.join("data.rose"))
                                .unwrap_or_else(|_| panic!("Failed to load AruaVfs at {}", path)),
                        ),
                        index_root_path,
                        "AruaVfs",
                    );
                }
                FilesystemDeviceConfig::TitanVfs(path) => {
                    let index_root_path = vfs_index_root_path(path);
                    log::info!("Loading game data from TitanVfs {}", path);
                    add_vfs_device(
                        &mut vfs_devices,
                        &mut base_path,
                        Box::new(
                            TitanVfsIndex::load(Path::new(path), &index_root_path.join("data.trf"))
                                .unwrap_or_else(|_| panic!("Failed to load TitanVfs at {}", path)),
                        ),
                        index_root_path,
                        "TitanVfs",
                    );
                }
                FilesystemDeviceConfig::Vfs(path) => {
                    let index_root_path = vfs_index_root_path(path);
                    log::info!("Loading game data from Vfs {}", path);
                    add_vfs_device(
                        &mut vfs_devices,
                        &mut base_path,
                        Box::new(
                            VfsIndex::load(Path::new(path))
                                .unwrap_or_else(|_| panic!("Failed to load Vfs at {}", path)),
                        ),
                        index_root_path,
                        "Vfs",
                    );
                }
                FilesystemDeviceConfig::IrosePh(path) => {
                    let index_root_path = vfs_index_root_path(path);
                    log::info!("Loading game data from iRosePH {}", path);
                    add_vfs_device(
                        &mut vfs_devices,
                        &mut base_path,
                        Box::new(
                            IrosePhVfsIndex::load(Path::new(path))
                                .unwrap_or_else(|_| panic!("Failed to load iRosePH VFS at {}", path)),
                        ),
                        index_root_path,
                        "iRosePH",
                    );
                }
            }
        }

        if vfs_devices.is_empty() {
            None
        } else {
            let vfs = Arc::new(VirtualFilesystem::new(vfs_devices));
            let base = match base_path {
                Some(path) if !path.as_os_str().is_empty() => path,
                _ => {
                    // Fallback to current directory, but log a warning if it fails
                    match std::env::current_dir() {
                        Ok(cwd) => cwd,
                        Err(e) => {
                            log::error!("[VFS] Failed to get current directory: {}. Save functionality may not work correctly.", e);
                            PathBuf::new() // Empty path as last resort
                        }
                    }
                }
            };
            log::info!("[VFS] Base path for saving: {:?}", base);
            Some((vfs, base))
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub ip: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".into(),
            port: 29000,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct GameConfig {
    pub data_version: String,
    pub network_version: String,
    pub ui_version: String,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            data_version: "irose".into(),
            network_version: "irose".into(),
            ui_version: "irose".into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum GraphicsModeConfig {
    #[serde(rename = "window")]
    Window { width: f32, height: f32 },
    #[serde(rename = "fullscreen")]
    Fullscreen,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct GraphicsConfig {
    pub mode: GraphicsModeConfig,
    pub passthrough_terrain_textures: bool,
    pub trail_effect_duration_multiplier: f32,
    pub disable_vsync: bool,
    pub use_new_terrain: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            mode: GraphicsModeConfig::Window {
                width: 1920.0,
                height: 1080.0,
            },
            passthrough_terrain_textures: false,
            trail_effect_duration_multiplier: 1.0,
            disable_vsync: false,
            use_new_terrain: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct SoundVolumeConfig {
    pub global: f32,
    pub background_music: f32,
    pub player_footstep: f32,
    pub player_combat: f32,
    pub other_footstep: f32,
    pub other_combat: f32,
    pub npc_sounds: f32,
    pub ui_sounds: f32,
}

impl Default for SoundVolumeConfig {
    fn default() -> Self {
        Self {
            global: 0.6,
            background_music: 0.15,
            player_footstep: 0.9,
            player_combat: 1.0,
            other_footstep: 0.5,
            other_combat: 0.5,
            npc_sounds: 0.6,
            ui_sounds: 0.5,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct SoundConfig {
    pub enabled: bool,
    pub volume: SoundVolumeConfig,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: SoundVolumeConfig::default(),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub account: AccountConfig,
    pub auto_login: AutoLoginConfig,
    pub filesystem: FilesystemConfig,
    pub game: GameConfig,
    pub graphics: GraphicsConfig,
    pub server: ServerConfig,
    pub sound: SoundConfig,
}

pub fn load_config(path: &Path) -> Config {
    let toml_str = match std::fs::read_to_string(path) {
        Ok(toml_str) => toml_str,
        Err(error) => {
            println!(
                "Failed to load configuration from {} with error: {}",
                path.to_string_lossy(),
                error
            );
            return Config::default();
        }
    };

    match toml::from_str(&toml_str) {
        Ok(config) => {
            println!("Read configuration from {}", path.to_string_lossy());
            config
        }
        Err(error) => {
            println!(
                "Failed to load configuration from {} with error: {}",
                path.to_string_lossy(),
                error
            );
            Config::default()
        }
    }
}

#[derive(Default)]
pub struct SystemsConfig {
    pub disable_player_command_system: bool,
    pub add_custom_systems: Option<Box<dyn FnOnce(&mut App)>>,
}

pub fn run_game(config: &Config, systems_config: SystemsConfig) {
    run_client(config, AppState::GameLogin, systems_config);
}

pub fn run_model_viewer(config: &Config) {
    run_client(config, AppState::ModelViewer, SystemsConfig::default());
}

pub fn run_zone_viewer(config: &Config, zone_id: Option<ZoneId>) {
    run_client(
        config,
        AppState::ZoneViewer,
        SystemsConfig {
            add_custom_systems: Some(Box::new(move |app| {
                app.world_mut().write_message(LoadZoneEvent::new(
                    zone_id.unwrap_or_else(|| ZoneId::new(1).unwrap()),
                ));
            })),
            ..Default::default()
        },
    );
}

/// Run the map editor mode
///
/// This launches the application in map editor mode, which allows editing
/// zone objects, terrain, and entity properties through an egui-based interface.
pub fn run_map_editor(config: &Config, zone_id: Option<ZoneId>) {
    run_client(
        config,
        AppState::MapEditor,
        SystemsConfig {
            add_custom_systems: Some(Box::new(move |app| {
                app.world_mut().write_message(LoadZoneEvent::new(
                    zone_id.unwrap_or_else(|| ZoneId::new(1).unwrap()),
                ));
            })),
            ..Default::default()
        },
    );
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
enum GameStages {
    ZoneChange,
    ZoneChangeFlush,
    AfterUpdate,
    DebugRenderPreFlush,
    DebugRender,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum GameSystemSets {
    UpdateCamera,
    Ui,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum UiSystemSets {
    UiDebugMenu,
    UiFirst,
    Ui,
    UiLast,
    UiDebug,
}

// System sets for ordering critical systems
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum ModelSystemSets {
    CharacterModelUpdate,
    CharacterModelAddCollider,
    PersonalStoreModel,
    PersonalStoreModelAddCollider,
    NpcModelUpdate,
    NpcModelAddCollider,
    ItemDropModel,
    ItemDropModelAddCollider,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum EffectSystemSets {
    AnimationEffect,
    Projectile,
    SpawnProjectile,
    PendingDamage,
    PendingSkillEffect,
    HitEvent,
    SpawnEffect,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum UiSystemOrdering {
    GameMouseInput,
    NameTagVisibility,
    MoveDestinationEffect,
}

fn run_client(config: &Config, app_state: AppState, mut systems_config: SystemsConfig) {
    log::info!(
        "[VFS INIT] Config has {} filesystem devices",
        config.filesystem.devices.len()
    );

    let (virtual_filesystem, base_path) =
        if let Some((vfs, base)) = config.filesystem.create_virtual_filesystem() {
            log::info!("[VFS INIT] Base path for saving: {:?}", base);
            (vfs, base)
        } else {
            log::error!("[VFS INIT] No filesystem devices configured, VFS initialization failed!");
            return;
        };

    let (window_width, window_height) =
        if let GraphicsModeConfig::Window { width, height } = config.graphics.mode {
            (width, height)
        } else {
            (1920.0, 1080.0)
        };

    let mut app = App::new();

    // OPTIMIZATION: Only clone once for VfsResource. VfsAssetReaderPlugin retrieves
    // the VFS from VfsResource during build, eliminating a redundant Arc clone.
    // Previously: 2 clones (one for plugin, one for resource)
    // Now: 1 clone (only for resource, plugin retrieves from resource)
    app.insert_resource(VfsResource {
        vfs: virtual_filesystem.clone(),
        base_path,
    })
    // Register VFS asset reader BEFORE DefaultPlugins (required for VFS-based asset loading)
    // VfsAssetReaderPlugin gets the VFS from VfsResource instead of holding its own Arc
    .add_plugins(VfsAssetReaderPlugin::new());

    // Initialise bevy engine
    app.add_plugins((
            bevy::prelude::DefaultPlugins
                .set(bevy::render::RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        backends: Some(Backends::all()),
                        // Keep problematic bindless features disabled for stability,
                        // but allow texture binding arrays needed by TerrainMaterial.
                        disabled_features: Some(
                            WgpuFeatures::BUFFER_BINDING_ARRAY
                                | WgpuFeatures::STORAGE_RESOURCE_BINDING_ARRAY
                                | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY
                                ,
                        ),
                        ..Default::default()
                    })),
                    synchronous_pipeline_compilation: false,
                    debug_flags: Default::default(),
                })
                .set(bevy::window::WindowPlugin {
                    primary_window: Some(Window {
                        title: "rose-offline-client".to_string(),
                        present_mode: if config.graphics.disable_vsync {
                            bevy::window::PresentMode::Immediate
                        } else {
                            bevy::window::PresentMode::Fifo
                        },
                        resolution: bevy::window::WindowResolution::new(
                            window_width as u32,
                            window_height as u32,
                        ),
                        mode: if matches!(config.graphics.mode, GraphicsModeConfig::Fullscreen) {
                            WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary)
                        } else {
                            WindowMode::Windowed
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::INFO,
                    filter: "wgpu=error,naga=error,offset_allocator=warn,rose_offline_client::map_editor=info".to_string(),
                    ..default()
                })
                .set(bevy::pbr::PbrPlugin::default()),
            bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            bevy::diagnostic::FrameTimeDiagnosticsPlugin::new(60),  // 60 frame history
        ));

    // Initialise 3rd party bevy plugins
    // Note: RapierConfiguration is no longer a Resource in Bevy 0.15
    // Configuration is now handled through the RapierPhysicsPlugin
    // bevy_egui bindless mode defaults to a 16-slot texture/sampler array.
    // With our custom WgpuSettings disabling PARTIALLY_BOUND_BINDING_ARRAY,
    // wgpu 27 validation requires all 16 items to be provided and can panic.
    // Disable bevy_egui bindless mode to use per-texture bind groups instead.
    app.add_plugins(bevy_egui::EguiPlugin {
        bindless_mode_array_size: None,
        ..Default::default()
    });
    // The main camera carries PrimaryEguiContext explicitly (spawned in
    // load_common_game_data). Disable bevy_egui's auto-creation so extra
    // cameras (e.g. the water reflection camera) never steal the primary
    // egui context - which would panic `EguiContexts::ctx_mut()` with
    // MultipleEntities.
    app.insert_resource(bevy_egui::EguiGlobalSettings {
        auto_create_primary_context: false,
        ..Default::default()
    });
    app.add_plugins(bevy_rapier3d::prelude::RapierPhysicsPlugin::<
        bevy_rapier3d::prelude::NoUserData,
    >::default());
    // Disabled: RapierDebugRenderPlugin (debug plugin)
    // Disabled: RenderDocPlugin (debug plugin)
    app.add_plugins(OddioPlugin);

    // Initialise rose stuff
    // Create channel for async zone loading
    let (tx, rx) = mpsc::channel();
    app.insert_resource(ZoneLoadChannelSender(tx));
    app.insert_resource(ZoneLoadChannelReceiver(std::sync::Mutex::new(rx)));
    app.init_resource::<Assets<ZoneLoaderAsset>>();
    log::info!("[ZONE LOADER] Channel for async zone loading created and registered");

    // Initialize memory tracking resource for zone loading
    app.init_resource::<MemoryTrackingResource>();

    app.init_resource::<RenderExtractionDiagnostics>();

    // Initialize terrain enhancement with procedural noise
    app.add_plugins(terrain::TerrainEnhancementPlugin);
    log::info!("[TERRAIN] Terrain enhancement plugin initialized with procedural noise");

    // Shadow map resolution matches Medium default (2 cascades x 2048).
    // Previously 4096 paid 4x texels before the user ever touched settings.
    app.insert_resource(DirectionalLightShadowMap { size: 2048 });

    // Deferred rendering (opaque renderer method)
    app.insert_resource(DefaultOpaqueRendererMethod::deferred());

    // Effect cache for performance - prevents reloading effect files from disk
    app.init_resource::<effect_loader::EffectCache>();

    app.register_asset_loader(ZmsAssetLoader)
        .init_asset::<ZmsMaterialNumFaces>()
        .register_asset_loader(ZmsNoSkinAssetLoader)
        .register_asset_loader(DdsImageLoader)
        .register_asset_loader(ExeResourceLoader)
        .init_asset::<ExeResourceCursor>()
        .register_asset_loader(DialogLoader)
        .init_asset::<Dialog>()
        .insert_resource(RenderConfiguration {
            passthrough_terrain_textures: config.graphics.passthrough_terrain_textures,
            trail_effect_duration_multiplier: config.graphics.trail_effect_duration_multiplier,
            use_new_terrain: config.graphics.use_new_terrain,
        })
        .insert_resource(ServerConfiguration {
            ip: config.server.ip.clone(),
            port: format!("{}", config.server.port),
            preset_username: Some(config.account.username.clone()),
            preset_password: Some(config.account.password.clone()),
            preset_server_id: config.auto_login.server_id,
            preset_channel_id: config.auto_login.channel_id,
            preset_character_name: config.auto_login.character_name.clone(),
            auto_login: config.auto_login.enabled,
        })
        .insert_resource(SoundSettings {
            enabled: config.sound.enabled,
            global_gain: config.sound.volume.global,
            gains: enum_map! {
                SoundCategory::BackgroundMusic => config.sound.volume.background_music,
                SoundCategory::PlayerFootstep => config.sound.volume.player_footstep,
                SoundCategory::PlayerCombat => config.sound.volume.player_combat,
                SoundCategory::OtherFootstep => config.sound.volume.other_footstep,
                SoundCategory::OtherCombat => config.sound.volume.other_combat,
                SoundCategory::NpcSounds => config.sound.volume.npc_sounds,
                SoundCategory::Ui => config.sound.volume.ui_sounds,
            },
        })
        .add_plugins((
            RoseAnimationPlugin,
            // CRITICAL: Add these to fix the panic and enable rendering
            ParticleMaterialPlugin,
            // ExtendedMaterial plugins for object, terrain, water, and effect mesh
            // Use custom RoseObjectMaterialPlugin which includes zone lighting support
            RoseObjectMaterialPlugin::default(),
        ));
    log::info!("[MATERIAL PLUGIN] RoseObjectExtension plugin registered successfully");

    app.add_plugins((MaterialPlugin::<
        ExtendedMaterial<StandardMaterial, RoseEffectExtension>,
    >::default(),));
    log::info!("[MATERIAL PLUGIN] RoseEffectExtension plugin registered successfully");

    // Register extension material shaders
    app.add_plugins(ExtensionMaterialPlugin);
    log::info!("[MATERIAL PLUGIN] ExtensionMaterialPlugin registered successfully");

    // Optional: Add these for full rendering support
    app.add_plugins((
        ZoneLightingPlugin,
        WorldUiRenderPlugin,
        RoseRenderPlugin,
        RoseScriptingPlugin,
        DebugInspectorPlugin,
    ));

    app.add_plugins((
        // REQUIRED: SkinnedMeshFixPlugin deferred-inserts SkinnedMesh components after mesh loading.
        // Without this, skinned meshes won't render correctly (bind group layout mismatch).
        render::SkinnedMeshFixPlugin,

        // Fish in water feature
        FishPlugin,
        // Birds in sky feature
        BirdPlugin,
    ));

    app.add_plugins((
        // Weather season system
        systems::season::SeasonPlugin,
        // Dirt/dash effect when characters run
        DirtDashPlugin,
        // Angelic wing spawning for flight system
        WingSpawnPlugin,
        // Wind particle effect for flying
        WindEffectPlugin,
        // Vegetation wind sway effect (grass, trees, leaves)
        VegetationSwayPlugin,
        // DISABLED: bevy_procedural_grass not compatible with Bevy 0.18
        // ProceduralGrassPlugin::default(),

        // Underwater rendering effect
        UnderwaterEffectPlugin,
        // Planar water reflections (mirrored camera + off-screen texture)
        WaterReflectionPlugin,
        // Procedural starry sky with moon lighting
        StarrySkyMaterialPlugin,
        // Blood effect system (spatter decals, gash wounds)
        blood_effect_plugin::BloodEffectPlugin,
    ));

    app.add_plugins((
        // Map editor system
        map_editor::MapEditorPlugin,
        // Old 2D procedural cloud material (DISABLED):
        // CloudMaterialPlugin,
        // New 3D volumetric cloud system:
        VolumetricCloudPlugin,
    ));
    // Setup state
    app.insert_state(app_state);

    app.add_message::<BankEvent>()
        .add_message::<BoardBoatEvent>()
        .add_message::<ChatBubbleEvent>()
        .add_message::<ChatboxEvent>()
        .add_message::<CharacterSelectEvent>()
        .add_message::<ClanDialogEvent>()
        .add_message::<ClientEntityEvent>()
        .add_message::<ConversationDialogEvent>()
        .add_message::<FlightToggleEvent>()
        .add_message::<GameConnectionEvent>()
        .add_message::<HitEvent>()
        .add_message::<LoginEvent>()
        .add_message::<LoadZoneEvent>()
        .add_message::<MessageBoxEvent>()
        .add_message::<MoveDestinationEffectEvent>()
        .add_message::<MoveSpeedSetEvent>()
        .add_message::<NetworkEvent>()
        .add_message::<NumberInputDialogEvent>()
        .add_message::<NpcStoreEvent>()
        .add_message::<PartyEvent>()
        .add_message::<PingRequestEvent>()
        .add_message::<PingResponseEvent>()
        .add_message::<PersonalStoreEvent>()
        .add_message::<PlayerCommandEvent>()
        .add_message::<QuestScrollEvent>()
        .add_message::<QuestTriggerEvent>()
        .add_message::<SystemFuncEvent>()
        .add_message::<DisembarkBoatEvent>()
        .add_message::<SpawnEffectEvent>()
        .add_message::<SpawnProjectileEvent>()
        .add_message::<UseItemEvent>()
        .add_message::<WorldConnectionEvent>()
        .add_message::<ZoneEvent>()
        .add_message::<ZoneLoadedFromVfsEvent>()
        .add_message::<UiSoundEvent>();

    app.add_systems(PostUpdate, ApplyDeferred);

    app.add_systems(
        PostUpdate,
        (ApplyDeferred,).in_set(GameStages::DebugRenderPreFlush),
    );

    // Camera systems use EguiContexts to check if egui wants pointer input
    app.add_systems(
        Update,
        (free_camera_system, orbit_camera_system)
            .in_set(GameSystemSets::UpdateCamera)
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );

    // Configure system ordering for model systems
    app.configure_sets(
        Update,
        (
            ModelSystemSets::CharacterModelUpdate,
            ModelSystemSets::CharacterModelAddCollider.after(ModelSystemSets::CharacterModelUpdate),
            ModelSystemSets::PersonalStoreModel.after(ModelSystemSets::CharacterModelAddCollider),
            ModelSystemSets::PersonalStoreModelAddCollider
                .after(ModelSystemSets::PersonalStoreModel),
            ModelSystemSets::NpcModelUpdate.after(ModelSystemSets::PersonalStoreModelAddCollider),
            ModelSystemSets::NpcModelAddCollider.after(ModelSystemSets::NpcModelUpdate),
            ModelSystemSets::ItemDropModel.after(ModelSystemSets::NpcModelAddCollider),
            ModelSystemSets::ItemDropModelAddCollider.after(ModelSystemSets::ItemDropModel),
        ),
    );

    // Configure system ordering for effect systems
    app.configure_sets(
        Update,
        (
            EffectSystemSets::AnimationEffect,
            EffectSystemSets::Projectile.after(EffectSystemSets::AnimationEffect),
            EffectSystemSets::SpawnProjectile.after(EffectSystemSets::AnimationEffect),
            EffectSystemSets::PendingDamage
                .after(EffectSystemSets::AnimationEffect)
                .after(EffectSystemSets::Projectile),
            EffectSystemSets::PendingSkillEffect
                .after(EffectSystemSets::AnimationEffect)
                .after(EffectSystemSets::Projectile),
            EffectSystemSets::HitEvent
                .after(EffectSystemSets::AnimationEffect)
                .after(EffectSystemSets::PendingSkillEffect)
                .after(EffectSystemSets::Projectile),
            EffectSystemSets::SpawnEffect
                .after(EffectSystemSets::AnimationEffect)
                .after(EffectSystemSets::HitEvent),
        ),
    );

    // Configure system ordering for UI systems
    app.configure_sets(
        Update,
        (
            UiSystemOrdering::GameMouseInput,
            UiSystemOrdering::NameTagVisibility.after(UiSystemOrdering::GameMouseInput),
            UiSystemOrdering::MoveDestinationEffect.after(UiSystemOrdering::GameMouseInput),
        ),
    );

    app.add_systems(
        Update,
        (
            auto_login_system,
            background_music_system,
            particle_sequence_system,
            particle_storage_buffer_update_system
                .after(particle_sequence_system)
                .run_if(resource_exists::<systems::DefaultParticleTexture>),
            effect_system,
            animation_sound_system,
            npc_idle_sound_system,
            character_model_update_system,
            character_model_add_collider_system,
        ),
    );
    app.add_systems(Update, memory_diagnostics_system);
    // name_tag_system uses EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(bevy_egui::EguiPrimaryContextPass, name_tag_system);
    // chat_bubble_spawn_system uses EguiContexts for text rendering - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(bevy_egui::EguiPrimaryContextPass, chat_bubble_spawn_system);
    // chat bubble update and cleanup systems
    app.add_systems(
        Update,
        (
            chat_bubble_update_system,
            chat_bubble_cleanup_system,
            chat_bubble_orphan_cleanup_system,
        ),
    );
    // monster chatter system for random NPC phrases
    app.add_systems(Update, (add_monster_chatter_system, monster_chatter_system));
    app.add_systems(
        Update,
        (
            personal_store_model_system,
            personal_store_model_add_collider_system,
            npc_model_update_system,
            npc_model_add_collider_system,
            item_drop_model_system,
            item_drop_model_add_collider_system,
            animation_effect_system,
            projectile_system,
            spawn_projectile_system,
        ),
    );

    app.add_systems(
        Update,
        (
            pending_damage_system,
            pending_skill_effect_system,
            hit_event_system,
            spawn_effect_system
                .after(visible_status_effects_system)
                .after(ui_debug_effect_list_system),
            visible_status_effects_system,
            move_destination_effect_system,
            damage_number_billboard_system,
            damage_number_animate_system,
            name_tag_update_healthbar_system,
            name_tag_visibility_system,
            name_tag_update_color_system,
            world_time_system,
            system_func_event_system,
            load_dialog_sprites_system,
            zone_time_system,
            // Toggle atmosphere based on time of day (disable at night for stars)
            // Must run after zone_time_system to get current time state
            toggle_atmosphere_based_on_time.after(zone_time_system),
            // Update starry sky night_factor from zone time state
            // Must run after zone_time_system and before update_starry_sky_system
            update_starry_sky_night_factor.after(zone_time_system),
            // DISABLED: color_grading_time_of_day_system conflicts with Bevy 0.16 Atmosphere
            // This system was applying time-based color grading (temperature/saturation changes)
            // which conflicts with the new atmospheric scattering system.
            // color_grading_time_of_day_system,
            directional_light_system,
            // Update terrain lighting based on zone lighting and time of day
            // Must run after zone_time_system to get current time state for intensity adjustment
            render::terrain_material::update_terrain_lighting_system.after(zone_time_system),
            // Starry sky material update - updates uniforms for twinkling and night factor
            // Runs after update_starry_sky_night_factor to use updated night_factor value
            update_starry_sky_system.after(update_starry_sky_night_factor),
        ),
    );
    // Separate add_systems call: the tuple above is already at Bevy's 20-system tuple limit
    // (see pitfalls/ecs-system-tuples.md). Sky/moon follow lives here to avoid E0277.
    app.add_systems(
        Update,
        (
            follow_sky_to_camera_system,
            moon_light_follow_camera_system,
        ),
    );
    // Must run after name_tag_visibility_system so the line-of-sight result
    // has the final say on name tag / chat bubble root visibility.
    app.add_systems(
        Update,
        world_ui_occlusion_system.after(name_tag_visibility_system),
    );

    // update_ui_resources uses EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(bevy_egui::EguiPrimaryContextPass, update_ui_resources);

    // ui_item_drop_name_system uses EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(bevy_egui::EguiPrimaryContextPass, ui_item_drop_name_system);

    // ui_message_box_system and ui_number_input_dialog_system use EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (ui_message_box_system, ui_number_input_dialog_system),
    );
    // ui_window_sound_system and ui_sound_event_system use EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (ui_window_sound_system, ui_sound_event_system),
    );

    // ui_debug_menu_system uses EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(bevy_egui::EguiPrimaryContextPass, ui_debug_menu_system);

    // Debug UI systems use EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (
            ui_debug_camera_info_system,
            ui_debug_client_entity_list_system,
            ui_debug_command_viewer_system,
            ui_debug_dialog_list_system,
            ui_debug_effect_list_system,
            ui_debug_entity_inspector_system,
            ui_debug_item_list_system,
            ui_debug_npc_list_system,
        ),
    );

    // DISABLED: app.add_systems(Update, ui_debug_physics_system); // Too many parameters for Bevy 0.15
    // More debug UI systems - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(bevy_egui::EguiPrimaryContextPass, ui_debug_render_system);
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        ui_debug_skill_list_system,
    );
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        ui_debug_zone_lighting_system,
    );
    app.add_systems(bevy_egui::EguiPrimaryContextPass, ui_debug_zone_list_system);
    app.add_systems(bevy_egui::EguiPrimaryContextPass, ui_debug_zone_time_system);
    // DISABLED: app.add_systems(Update, ui_debug_diagnostics_system);

    // character_model_blink_system in PostUpdate to avoid any conflicts with model destruction
    // e.g. through the character select exit system.
    app.add_systems(PostUpdate, character_model_blink_system);

    // vehicle_model_system in after ::Update but before ::PostUpdate to avoid any conflicts,
    // with model destruction but to also be before global transform is calculated.
    app.add_systems(PostUpdate, (vehicle_model_system, vehicle_sound_system));

    // Configure vehicle system ordering
    app.configure_sets(PostUpdate, GameStages::AfterUpdate);
    app.add_systems(PostUpdate, vehicle_sound_system);

    // Run zone change system just before physics sync which is after Update
    // DIAGNOSTIC: Added explicit system ordering to ensure proper event flow:
    // zone_loader_system → zone_loaded_from_vfs_system → game_zone_change_system
    // CRITICAL FIX: game_zone_change_system MUST run after zone loading completes
    // to ensure ZoneEvent::Loaded events are processed correctly.
    app.add_systems(
        Update,
        (
            zone_loader_system,
            // zone_loaded_from_vfs_system runs after zone_loader_system to process the events it sends
            zone_loaded_from_vfs_system.after(zone_loader_system),
        ),
    );

    app.add_systems(
        PostUpdate,
        force_zone_visibility_system
            .after(VisibilitySystems::VisibilityPropagate)
            .before(VisibilitySystems::CheckVisibility),
    );

    app.add_systems(
        Update,
        (
            // CRITICAL FIX: game_zone_change_system must run after BOTH zone systems
            // to ensure it sees the ZoneEvent::Loaded events properly
            game_zone_change_system
                .after(zone_loader_system)
                .after(zone_loaded_from_vfs_system),
        ),
    );

    // Zone Viewer
    app.add_systems(OnEnter(AppState::ZoneViewer), zone_viewer_enter_system);

    // Map Editor
    app.add_systems(
        OnEnter(AppState::MapEditor),
        map_editor::map_editor_enter_system,
    );
    app.add_systems(
        OnExit(AppState::MapEditor),
        map_editor::map_editor_exit_system,
    );

    // Model Viewer, we avoid deleting any entities during CoreStage::Update by using a custom
    // stage which runs after Update. We cannot run before Update because the on_enter system
    // below will have not run yet.
    app.add_systems(OnEnter(AppState::ModelViewer), model_viewer_enter_system);
    app.add_systems(OnExit(AppState::ModelViewer), model_viewer_exit_system);
    app.add_systems(
        PostUpdate,
        model_viewer_system.run_if(in_state(AppState::ModelViewer)),
    );

    // Game Login
    app.add_systems(OnEnter(AppState::GameLogin), login_state_enter_system)
        .add_systems(OnExit(AppState::GameLogin), login_state_exit_system);

    // In bevy_egui 0.39, UI systems must run in EguiPrimaryContextPass schedule
    // (not Update) to ensure the egui pass has been started before using ctx
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (login_system.before(login_event_system), login_event_system)
            .run_if(in_state(AppState::GameLogin)),
    );

    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (ui_login_system, ui_server_select_system)
            .run_if(in_state(AppState::GameLogin))
            .in_set(UiSystemSets::Ui)
            .after(login_system)
            .before(login_event_system),
    );

    // Game Character Select
    app.add_systems(
        OnEnter(AppState::GameCharacterSelect),
        character_select_enter_system,
    )
    .add_systems(
        OnExit(AppState::GameCharacterSelect),
        character_select_exit_system,
    );

    app.init_resource::<CharacterSelectInputState>();
    // character_select_system uses EguiContexts for UI dialogs - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        character_select_system.run_if(in_state(AppState::GameCharacterSelect)),
    );
    // character_select_models_system and character_select_event_system don't use EguiContexts
    app.add_systems(
        Update,
        (
            character_select_models_system,
            character_select_event_system,
        )
            .run_if(in_state(AppState::GameCharacterSelect)),
    );

    // UI systems for character select - must run in EguiPrimaryContextPass for bevy_egui 0.39
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (
            ui_character_create_system,
            ui_character_select_system,
            ui_character_select_name_tag_system,
        )
            .run_if(in_state(AppState::GameCharacterSelect)),
    );
    // character_select_input_system uses EguiContexts to check if egui wants pointer input
    // This can stay in Update since it only queries egui state, doesn't render
    app.add_systems(
        Update,
        character_select_input_system
            .run_if(in_state(AppState::GameCharacterSelect))
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );

    // Game
    app.init_resource::<UiStateDragAndDrop>()
        .init_resource::<UiStateWindows>()
        .init_resource::<UiStateDebugWindows>()
        .init_resource::<UiStateAdminMenu>()
        .init_resource::<PingState>()
        .init_resource::<ClientEntityList>()
        .init_resource::<DebugRenderConfig>()
        .init_resource::<WorldTime>()
        .init_resource::<ZoneTime>()
        .init_resource::<SelectedTarget>()
        .init_resource::<NameTagSettings>()
        .init_resource::<DepthOfFieldSettings>()
        .init_resource::<ui::PostProcessingSettings>()
        .init_resource::<ui::StarrySkyRenderSettings>()
        .init_resource::<WaterSettings>()
        .init_resource::<FlightSettings>()
        .init_resource::<WindSettings>()
        .init_resource::<WindState>()
        .init_resource::<MonsterChatterPhrases>()
        .init_resource::<AtmosphereState>()
        .init_resource::<graphics::GraphicsSettings>();

    app.add_systems(OnEnter(AppState::Game), game_state_enter_system);

    // Spawn sky systems on startup
    // app.add_systems(PostStartup, (spawn_starry_sky_and_moon, spawn_cloud_layer));
    app.add_systems(
        PostStartup,
        (spawn_starry_sky_and_moon, spawn_volumetric_clouds),
    );

    // System to apply depth of field settings from the resource to the camera
    app.add_systems(Update, apply_depth_of_field_settings);

    // System to apply post-processing settings from the resource to the camera
    app.add_systems(Update, apply_post_processing_settings);

    // System to apply water settings from the resource to water materials
    app.add_systems(Update, apply_water_settings);

    // Graphics settings apply systems
    app.add_systems(
        PostUpdate,
        (
            graphics::apply_color_grading_system,
            graphics::apply_shadow_quality_system,
            graphics::apply_tonemapping_system,
            graphics::apply_bloom_system,
            graphics::apply_ssao_system,
            graphics::apply_smaa_system,
            graphics::apply_fxaa_system,
            graphics::apply_motion_blur_system,
            graphics::apply_dof_enabled_system,
            graphics::apply_view_distance_system,
            graphics::apply_texture_quality_system,
            graphics::apply_shadow_filtering_system,
            graphics::apply_msaa_system,
            graphics::apply_ambient_light_system,
        ),
    );

    // Game systems - part 1
    app.add_systems(
        Update,
        (
            ability_values_system,
            clan_system,
            command_system,
            facing_direction_system,
            update_position_system.after(command_system),
            // monster_separation_system DISABLED: it displaced monsters client-side
            // only (the server has no separation), desyncing combat positions with
            // no reconciliation path. Result: player swung at ghosts (no damage,
            // idle monster) or chased forever on open ground. Stacked monsters may
            // visually overlap again; correctness wins over looks.
            collision_height_only_system,
            // CRITICAL: collision_player_system_join_zone must run BEFORE collision_player_system
            // - join_zone uses a long raycast (Y=100000) to find initial ground height on spawn
            // - collision_player_system uses short raycast for continuous terrain following
            // Using Added<CollisionPlayer> filter ensures join_zone only runs once on spawn
            collision_player_system_join_zone.before(collision_player_system),
            // Deterministic order: command -> move -> collide. Without this the
            // tuple runs unordered and a wall-collision Stop can race (and win
            // over) an attack chase set by command_system in the same tick.
            collision_player_system
                .after(command_system)
                .after(update_position_system),
            cooldown_system,
            client_entity_event_system,
            // Global wind simulation and vegetation synchronization
            wind_update_system,
            sync_vegetation_wind_system.after(wind_update_system),
            // Flight systems - ensure_flight_state_system runs before flight_toggle_system
            ensure_flight_state_system,
            flight_toggle_system.after(ensure_flight_state_system),
            flight_movement_system.after(flight_toggle_system),
            // Flight pose blend update system - updates pose_blend value on FlightState
            flight_pose_blend_update_system.after(flight_toggle_system),
            // Flight pose system applies visual-only rotations to character model parts
            // Runs after facing_direction_system and character_model_update_system
            flight_pose_system
                .after(facing_direction_system)
                .after(flight_toggle_system)
                .after(character_model_update_system),
            // Move speed command system
            move_speed_set_system,
        )
            .run_if(in_state(AppState::Game)),
    );

    // Sailing systems
    app.add_systems(
        Update,
        ensure_boat_state_system.run_if(in_state(AppState::Game)),
    );
    app.add_systems(
        Update,
        boat_toggle_system
            .run_if(in_state(AppState::Game))
            .after(ensure_boat_state_system),
    );
    app.add_systems(
        Update,
        remote_boat_sync_system
            .run_if(in_state(AppState::Game))
            .after(boat_toggle_system)
            .after(update_position_system),
    );
    app.add_systems(
        Update,
        ensure_boat_wake_emitter_system
            .run_if(in_state(AppState::Game))
            .after(boat_toggle_system)
            .after(remote_boat_sync_system),
    );
    app.add_systems(
        Update,
        sailing_movement_system
            .run_if(in_state(AppState::Game))
            .after(boat_toggle_system)
            .after(wind_update_system),
    );
    app.add_systems(
        Update,
        sail_animation_system
            .run_if(in_state(AppState::Game))
            .after(sailing_movement_system)
            .after(remote_boat_sync_system),
    );
    app.add_systems(
        Update,
        boat_buoyancy_system
            .run_if(in_state(AppState::Game))
            .after(sailing_movement_system)
            .after(remote_boat_sync_system)
            .after(facing_direction_system),
    );
    app.add_systems(
        Update,
        sail_camera_system
            .run_if(in_state(AppState::Game))
            .after(boat_toggle_system),
    );
    app.add_systems(
        Update,
        boat_wake_spawn_system
            .run_if(in_state(AppState::Game))
            .after(sailing_movement_system)
            .after(remote_boat_sync_system),
    );
    app.add_systems(
        Update,
        boat_wake_update_system
            .run_if(in_state(AppState::Game))
            .after(boat_wake_spawn_system),
    );
    app.add_systems(
        Update,
        zone_content::npcs::spawn_dock_npcs_system
            .run_if(in_state(AppState::Game))
            .after(ensure_boat_state_system),
    );
    app.add_systems(
        Update,
        zone_content::monsters::spawn_sea_monsters_system
            .run_if(in_state(AppState::Game))
            .after(ensure_boat_state_system),
    );
    app.add_systems(
        Update,
        zone_content::monsters::sea_monster_ai_system
            .run_if(in_state(AppState::Game))
            .after(boat_toggle_system)
            .after(sailing_movement_system),
    );
    app.add_systems(
        Update,
        zone_content::boats::spawn_random_boats_system
            .run_if(in_state(AppState::Game))
            .after(ensure_boat_state_system),
    );
    app.add_systems(
        Update,
        zone_content::boats::npc_boat_movement_system
            .run_if(in_state(AppState::Game))
            .after(ensure_boat_wake_emitter_system)
            .before(boat_buoyancy_system),
    );
    app.add_systems(
        Update,
        zone_content::docks::spawn_docks_system
            .run_if(in_state(AppState::Game))
            .after(ensure_boat_state_system),
    );
    app.add_systems(
        Update,
        ensure_boat_sound_state_system
            .run_if(in_state(AppState::Game))
            .after(boat_toggle_system),
    );
    app.add_systems(
        Update,
        boat_loop_sound_update_system
            .run_if(in_state(AppState::Game))
            .after(sailing_movement_system),
    );
    app.add_systems(
        Update,
        boat_one_shot_sound_system
            .run_if(in_state(AppState::Game))
            .after(sailing_movement_system),
    );

    // Game systems - part 2
    app.add_systems(
        Update,
        (
            use_item_event_system,
            status_effect_system,
            passive_recovery_system,
            quest_trigger_system,
        )
            .run_if(in_state(AppState::Game)),
    );
    // game_mouse_input_system uses EguiContexts to check if egui wants pointer input
    // This can stay in Update since it only queries egui state, doesn't render
    // game_keyboard_input_system uses EguiContexts to skip input while typing in UI.
    app.add_systems(
        Update,
        (
            game_mouse_input_system.after(bevy_egui::EguiPreUpdateSet::InitContexts),
            game_keyboard_input_system.after(bevy_egui::EguiPreUpdateSet::InitContexts),
        ),
    );

    // UI systems - part 1 (must run in EguiPrimaryContextPass for bevy_egui 0.39)
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (
            ui_admin_menu_system,
            ui_bank_system,
            ui_chatbox_system,
            ui_character_info_system,
            ui_clan_system,
            ui_create_clan_system,
            ui_inventory_system,
            ui_game_menu_system,
            ui_hotbar_system,
            ui_minimap_system,
            ui_npc_store_system,
            ui_party_system,
            ui_party_option_system,
            ui_personal_store_system,
            ui_player_info_system,
            ui_quest_list_system,
        )
            .run_if(in_state(AppState::Game)),
    );
    app.add_systems(
        Update,
        admin_menu_keyboard_system.run_if(in_state(AppState::Game)),
    );

    // UI systems - part 2 (must run in EguiPrimaryContextPass for bevy_egui 0.39)
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        (
            ui_respawn_system,
            ui_selected_target_system,
            ui_skill_list_system,
            ui_skill_tree_system,
            ui_settings_system,
            ui_status_effects_system,
            conversation_dialog_system,
        )
            .run_if(in_state(AppState::Game)),
    );
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        ui_sailing_hud_system.run_if(in_state(AppState::Game)),
    );

    if !systems_config.disable_player_command_system {
        app.add_systems(
            Update,
            player_command_system.run_if(in_state(AppState::Game)),
        );
    }

    // ui_drag_and_drop_system uses EguiContexts - must run in EguiPrimaryContextPass for bevy_egui 0.39
    // Must run AFTER all UI systems that handle drop targets, otherwise it takes dragged_item
    // before those systems can detect and process the drop (see pitfalls/skill-bar-ui.md)
    app.add_systems(
        bevy_egui::EguiPrimaryContextPass,
        ui_drag_and_drop_system
            .after(ui_npc_store_system)
            .after(ui_hotbar_system)
            .after(ui_inventory_system)
            .after(ui_personal_store_system)
            .after(ui_bank_system)
            .after(ui_skill_list_system)
            .after(ui_skill_tree_system),
    );

    // Setup network
    let (network_thread_tx, network_thread_rx) =
        tokio::sync::mpsc::unbounded_channel::<NetworkThreadMessage>();
    let network_thread = std::thread::spawn(move || run_network_thread(network_thread_rx));
    app.insert_resource(NetworkThread::new(network_thread_tx.clone()));

    // Run network systems before Update, so we can add/remove entities
    app.add_systems(
        PreUpdate,
        (
            login_connection_system,
            world_connection_system,
            game_connection_system.run_if(resource_exists::<CurrentZone>),
        ),
    );

    app.add_systems(
        PostStartup,
        load_common_game_data.after(bevy_egui::EguiStartupSet::InitContexts),
    );

    // Setup egui fonts after camera with PrimaryEguiContext is spawned
    app.add_systems(PostStartup, setup_egui_fonts.after(load_common_game_data));

    // Create default particle texture before particle systems run
    app.add_systems(PostStartup, create_default_particle_texture);
    app.add_systems(PostStartup, setup_boat_wake_assets);
    app.add_systems(PostStartup, setup_boat_sound_assets);

    // DIAGNOSTIC: Print diagnostic summary on startup
    app.add_systems(PostStartup, print_diagnostic_summary);

    if let Some(app_builder) = systems_config.add_custom_systems.take() {
        app_builder(&mut app);
    }

    match config.game.network_version.as_str() {
        "irose" => {
            app.add_systems(PostUpdate, network_thread_system);
        }
        "custom" => {}
        unknown => panic!("Unknown game network version {}", unknown),
    };

    match config.game.ui_version.as_str() {
        "irose" => {
            // Run after load_common_game_data spawns the camera with PrimaryEguiContext
            app.add_systems(PostStartup, load_ui_resources.after(load_common_game_data));
        }
        "custom" => {}
        unknown => panic!("Unknown game ui version {}", unknown),
    };

    match config.game.data_version.as_str() {
        "irose" => {
            app.add_systems(Startup, load_game_data_irose);
        }
        "custom" => {}
        unknown => panic!("Unknown game data version {}", unknown),
    };

    app.configure_sets(
        PostUpdate,
        (GameStages::AfterUpdate,).before(PhysicsSet::SyncBackend),
    );

    app.configure_sets(
        PostUpdate,
        (
            GameStages::ZoneChange,
            GameStages::ZoneChangeFlush,
            GameStages::AfterUpdate,
        )
            .before(PhysicsSet::SyncBackend),
    );

    app.configure_sets(
        PostUpdate,
        (GameStages::DebugRenderPreFlush, GameStages::DebugRender).chain(),
    );

    // CRITICAL FIX: Use Bevy's default ordering for internal systems
    // Manual ordering of internal sets like VisibilityPropagate can break engine logic
    app.configure_sets(
        PostUpdate,
        GameStages::AfterUpdate.before(TransformSystems::Propagate),
    );

    app.configure_sets(
        PostUpdate,
        VisibilitySystems::VisibilityPropagate.after(TransformSystems::Propagate),
    );
    app.configure_sets(
        PostUpdate,
        VisibilitySystems::CheckVisibility.after(VisibilitySystems::VisibilityPropagate),
    );
    app.configure_sets(
        PostUpdate,
        GameStages::DebugRenderPreFlush.after(VisibilitySystems::CheckVisibility),
    );

    app.configure_sets(
        Update,
        (
            UiSystemSets::UiDebugMenu,
            UiSystemSets::UiFirst,
            UiSystemSets::Ui,
            UiSystemSets::UiLast,
            UiSystemSets::UiDebug,
        )
            .in_set(GameSystemSets::Ui),
    );

    app.configure_sets(Update, (GameSystemSets::UpdateCamera, GameSystemSets::Ui));

    app.run();

    network_thread_tx.send(NetworkThreadMessage::Exit).ok();
    network_thread.join().ok();
}

fn load_game_data_irose(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    _asset_server: Res<AssetServer>,
) {
    let string_database = rose_data_irose::get_string_database(&vfs_resource.vfs, 1)
        .expect("Failed to load string database");

    let items = Arc::new(
        rose_data_irose::get_item_database(&vfs_resource.vfs, string_database.clone())
            .expect("Failed to load item database"),
    );
    let npcs = Arc::new(
        rose_data_irose::get_npc_database(
            &vfs_resource.vfs,
            string_database.clone(),
            &NpcDatabaseOptions {
                load_frame_data: false,
            },
        )
        .expect("Failed to load npc database"),
    );
    let skills = Arc::new(
        rose_data_irose::get_skill_database(&vfs_resource.vfs, string_database.clone())
            .expect("Failed to load skill database"),
    );
    let character_motion_database = Arc::new(
        rose_data_irose::get_character_motion_database(
            &vfs_resource.vfs,
            &CharacterMotionDatabaseOptions {
                load_frame_data: false,
            },
        )
        .expect("Failed to load character motion list"),
    );
    let zone_list = Arc::new(
        rose_data_irose::get_zone_list(&vfs_resource.vfs, string_database.clone())
            .expect("Failed to load zone list"),
    );

    // Initialize ZoneLoader with zone_list
    log::info!("[GameData] Initializing ZoneLoader with zone_list");
    zone_loader::ZoneLoader::init_zone_list(zone_list.clone());
    log::info!("[GameData] ZoneLoader initialized successfully");

    let sounds = rose_data_irose::get_sound_database(&vfs_resource.vfs)
        .expect("Failed to load sound database");

    commands.insert_resource(SoundCache::new(sounds.len()));

    commands.insert_resource(GameData {
        ability_value_calculator: rose_game_irose::data::get_ability_value_calculator(
            items.clone(),
            skills.clone(),
            npcs.clone(),
        ),
        animation_event_flags: {
            let flags = rose_data_irose::get_animation_event_flags();
            if flags.is_empty() {
                log::warn!("Animation event flags are empty! Gameplay effects may not trigger.");
            } else {
                log::info!("Loaded {} animation event flags", flags.len());
            }
            flags
        },
        character_motion_database,
        client_strings: rose_data_irose::get_client_strings(string_database.clone())
            .expect("Failed to load client strings"),
        data_decoder: rose_data_irose::get_data_decoder(),
        effect_database: {
            let db = rose_data_irose::get_effect_database(&vfs_resource.vfs)
                .expect("Failed to load effect database");
            if db.is_empty() {
                log::warn!("Effect database is empty! Visual effects will not appear.");
            } else {
                log::info!("Loaded {} effects from effect database", db.len());
            }
            db
        },
        items,
        job_class: Arc::new(
            rose_data_irose::get_job_class_database(&vfs_resource.vfs, string_database.clone())
                .expect("Failed to load job class database"),
        ),
        npcs,
        quests: Arc::new(
            rose_data_irose::get_quest_database(&vfs_resource.vfs, string_database.clone())
                .expect("Failed to load quest database"),
        ),
        skills,
        skybox: rose_data_irose::get_skybox_database(&vfs_resource.vfs)
            .expect("Failed to load skybox database"),
        sounds,
        status_effects: Arc::new(
            rose_data_irose::get_status_effect_database(&vfs_resource.vfs, string_database.clone())
                .expect("Failed to load status effect database"),
        ),
        string_database,
        zone_list,
        ltb_event: vfs_resource
            .vfs
            .read_file::<LtbFile, _>("3DDATA/EVENT/ULNGTB_CON.LTB")
            .expect("Failed to load event language file"),
        zsc_event_object: vfs_resource
            .vfs
            .read_file::<ZscFile, _>("3DDATA/SPECIAL/EVENT_OBJECT.ZSC")
            .expect("Failed to load 3DDATA/SPECIAL/EVENT_OBJECT.ZSC"),
        zsc_special_object: vfs_resource
            .vfs
            .read_file::<ZscFile, _>("3DDATA/SPECIAL/LIST_DECO_SPECIAL.ZSC")
            .expect("Failed to load 3DDATA/SPECIAL/LIST_DECO_SPECIAL.ZSC"),
        stb_morph_object: vfs_resource
            .vfs
            .read_file::<StbFile, _>("3DDATA/STB/LIST_MORPH_OBJECT.STB")
            .expect("Failed to load 3DDATA/STB/LIST_MORPH_OBJECT.STB"),
        character_select_positions: vec![
            Transform::from_translation(Vec3::new(5205.0, 1.0, -5205.0))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5202.70, 1.0, -5206.53))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5200.00, 1.0, -5207.07))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5197.30, 1.0, -5206.53))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5195.00, 1.0, -5205.00))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
        ],
    });
}

fn load_common_game_data(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<bevy::pbr::StandardMaterial>>,
    mut scattering_mediums: ResMut<Assets<bevy::light::atmosphere::ScatteringMedium>>,
) {


    commands.insert_resource(SpecularTexture {
        image: asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"),
    });

    // Preload the login screen camera animation to prevent race condition
    // where camera shows wrong angle on initial load
    let login_camera_animation_handle = asset_server.load("3DDATA/TITLE/CAMERA01_INTRO01.ZMO");
    commands.insert_resource(LoginCameraAnimation {
        handle: login_camera_animation_handle,
    });
    info!("[load_common_game_data] Preloaded login camera animation asset");

    commands.insert_resource(
        ModelLoader::new(
            vfs_resource.vfs.clone(),
            game_data.character_motion_database.clone(),
            game_data.effect_database.clone(),
            game_data.items.clone(),
            game_data.npcs.clone(),
            asset_server.load("3DDATA/EFFECT/TRAIL.DDS"),
            asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"),
            EffectCache::default(),
        )
        .expect("Failed to create model loader"),
    );


    let camera_entity = commands
        .spawn((
            Camera3d::default(),
            Msaa::Off, // Required for SSAO and TAA compatibility
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgb(0.0, 0.0, 0.02)), // Near-black for star visibility
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: std::f32::consts::PI / 4.0,
                near: 0.1,
                // Sky sphere now follows the camera (radius 4000), so 8000 comfortably
                // contains sky + shadow range + fog volume. Matches view_distance
                // mapping (500m default * 16). Previously 100000 destroyed depth
                // precision and Hi-Z efficiency.
                far: 8000.0,
                aspect_ratio: 16.0 / 9.0,
                ..default()
            }),
            Transform::from_translation(Vec3::new(5200.0, 30.0, -5180.0))
                .looking_at(Vec3::new(5200.0, 10.0, -5230.0), Vec3::Y),
            GlobalTransform::default(),
            // Primary Egui Context - required for bevy_egui 0.32+
            PrimaryEguiContext,
            // Add Tonemapping - REQUIRED for HDR to work properly with depth of field
            bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
            // Add Bloom - enhances the depth of field effect visibility
            Bloom::NATURAL,
            // Shadow filtering - Gaussian for high-quality soft shadows
            ShadowFilteringMethod::Gaussian,
            // NOTE: SMAA / SSR / MotionBlur / AutoExposure / CAS are NOT spawned by
            // default. They are inserted on demand by graphics apply systems when the
            // user enables them (SMAA/motion-blur have settings; SSR/AutoExposure/CAS
            // stay off until a future settings toggle wires them). Previously all were
            // stacked at startup for a ~6-pass fullscreen cost even on Low.
            // Prepasses for depth (required for some effects and GPU occlusion culling)
            DepthPrepass,
            // DeferredPrepass is REQUIRED with DefaultOpaqueRendererMethod::deferred():
            // without it the view gets Opaque3dPrepass phases but no Opaque3dDeferred
            // phases, and Bevy 0.18.1 panics in queue_prepass_material_meshes
            // (prepass/mod.rs unwrap) as soon as any opaque deferred material
            // (e.g. --new-terrain StandardMaterial) is visible. It also lets the
            // deferred G-buffer pass replace the duplicate forward prepass.
            DeferredPrepass,
            // GPU Occlusion Culling - culls objects hidden behind other objects to improve performance
            OcclusionCulling,
            // Underwater state tracking for underwater rendering effect
            CameraUnderwaterState::default(),
        ))
        .id();

    commands.entity(camera_entity).insert((
        // Environment Map Light for richer PBR reflections and lighting
        // The DDS loader is configured to load this texture as a cubemap when the #cube label is used
        // Intensity lowered 150 -> 100: 150 washed out shadows (see pitfalls/rendering-camera.md #5).
        EnvironmentMapLight {
            diffuse_map: asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS#cube"),
            specular_map: asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS#cube"),
            intensity: 100.0,
            ..default()
        },
        // Render layers 0 and 1: layer 0 is the world, layer 1 is water
        // (kept separate so the reflection camera can exclude water).
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
    ));
    // Insert additional components separately to avoid tuple size limit
    // DEBUG: Disable atmosphere when testing starry sky
    // When FORCE_NIGHT_MODE is true in starry_sky_material.rs, don't add atmosphere
    const DEBUG_DISABLE_ATMOSPHERE: bool = false;

    if !DEBUG_DISABLE_ATMOSPHERE {
        // Bevy 0.19: Atmosphere is a standalone entity (nearest one wins for
        // rendering); the camera only carries AtmosphereSettings to enable it
        // for its view. The day/night toggle system spawns/despawns the entity.
        commands.spawn(Atmosphere::earth(
            scattering_mediums.add(bevy::light::atmosphere::ScatteringMedium::default()),
        ));
        commands.entity(camera_entity).insert((
            // Bevy 0.19 built-in atmospheric scattering for realistic sky
            AtmosphereSettings::default(),
            // Depth of Field: Gaussian default (cheaper than Bokeh). Bokeh + CoC 64
            // is available via settings but not the startup cost.
            DepthOfField {
                mode: DepthOfFieldMode::Gaussian,
                focal_distance: 10.0,   // Focus 10 meters away
                aperture_f_stops: 3.3,  // f/3.3 aperture
                sensor_height: 0.01866, // Super 35 format (default)
                max_circle_of_confusion_diameter: 32.0,
                max_depth: 2000.0, // Max depth range
            },
            // VolumetricFog: 64 steps = Bevy default (was 128 = 2x raymarch cost).
            VolumetricFog {
                ambient_intensity: 0.1,
                jitter: 0.0,
                step_count: 64,
                ..default()
            },
            // SSAO Medium matches GraphicsSettings default (was Ultra at startup).
            ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                constant_object_thickness: 0.25, // Adjust if AO is too strong/weak
            },
        ));
    }
    info!(
        "[CAMERA] Camera entity spawned with id: {:?}, position: ~5120.0, 100.0, -5120.0 (game world center)",
        camera_entity
    );

    commands.insert_resource(DamageDigitsSpawner::load(
        &asset_server,
        &mut meshes,
        &mut standard_materials,
    ));
}

/// Setup egui fonts - runs after camera with PrimaryEguiContext is spawned
fn setup_egui_fonts(mut egui_context: EguiContexts) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Ubuntu-M".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "fonts/Ubuntu-M.ttf"
        ))),
    );

    fonts
        .families
        .entry(egui::FontFamily::Name("Ubuntu-M".into()))
        .or_default()
        .insert(0, "Ubuntu-M".to_owned());

    egui_context.ctx_mut().unwrap().set_fonts(fonts);

    let ctx = egui_context.ctx_mut().unwrap();
    let mut style = (*ctx.style()).clone();
    style.interaction.tooltip_delay = 0.05;
    ctx.set_style(style);
}

/// Diagnostic summary system
/// Prints a comprehensive diagnostic summary on startup
fn print_diagnostic_summary(
    cameras: Query<&Camera>,
    meshes: Query<&Mesh3d>,
    render_diagnostics: Res<RenderExtractionDiagnostics>,
) {
    info!("=== BEVY 0.14.2 DIAGNOSTIC SUMMARY ===");
    info!(
        "Active cameras: {}",
        cameras.iter().filter(|c| c.is_active).count()
    );
    info!("Total mesh entities: {}", meshes.iter().count());
    info!(
        "Main world meshes tracked: {}",
        render_diagnostics.main_world_mesh_count
    );
    info!("=======================================");
}

/// System to apply depth of field settings from the resource to the camera.
/// Disabling REMOVES the component so the DoF pass is skipped. Previously this only
/// switched Bokeh->Gaussian, which still ran the full DoF pass.
fn apply_depth_of_field_settings(
    dof_settings: Res<DepthOfFieldSettings>,
    // NOTE: reflection camera excluded (see graphics/apply_systems.rs invariant).
    camera_query: Query<
        (Entity, Option<&DepthOfField>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
    mut commands: Commands,
) {
    use bevy::ecs::change_detection::DetectChanges;

    // Only update if settings have changed
    if !dof_settings.is_changed() {
        return;
    }

    for (entity, dof) in camera_query.iter() {
        if dof_settings.enabled {
            match dof {
                Some(_) => {
                    commands.entity(entity).insert(DepthOfField {
                        mode: dof_settings.mode,
                        focal_distance: dof_settings.focal_distance,
                        aperture_f_stops: dof_settings.aperture_f_stops,
                        sensor_height: dof_settings.sensor_height,
                        max_circle_of_confusion_diameter:
                            dof_settings.max_circle_of_confusion_diameter,
                        max_depth: dof_settings.max_depth,
                    });
                }
                None => {
                    commands.entity(entity).insert(DepthOfField {
                        mode: dof_settings.mode,
                        focal_distance: dof_settings.focal_distance,
                        aperture_f_stops: dof_settings.aperture_f_stops,
                        sensor_height: dof_settings.sensor_height,
                        max_circle_of_confusion_diameter:
                            dof_settings.max_circle_of_confusion_diameter,
                        max_depth: dof_settings.max_depth,
                    });
                }
            }
        } else if dof.is_some() {
            commands.entity(entity).remove::<DepthOfField>();
        }
    }
}

/// System to apply post-processing settings from the resource to the camera
/// This allows live toggling of bloom, SSAO, volumetric fog, and color grading via the Settings UI
fn apply_post_processing_settings(
    post_process_settings: Res<ui::PostProcessingSettings>,
    // NOTE: reflection camera excluded (see graphics/apply_systems.rs invariant).
    mut camera_query: Query<(
        Entity,
        Option<&mut Bloom>,
        Option<&mut ScreenSpaceAmbientOcclusion>,
        Option<&mut VolumetricFog>,
        Option<&mut ColorGrading>,
    ), (With<Camera>, Without<crate::render::WaterReflectionCamera>)>,
    mut commands: Commands,
) {
    use bevy::ecs::change_detection::DetectChanges;

    // Only update if settings have changed
    if !post_process_settings.is_changed() {
        return;
    }

    for (entity, bloom, ssao, volumetric_fog, _color_grading) in camera_query.iter_mut() {
        // Handle Bloom (insert/remove so the pass is skipped when off).
        // NOTE: GraphicsSettings.bloom is the primary owner (PostUpdate wins on
        // simultaneous ticks); this page mirrors it. Intensity is honored here.
        if post_process_settings.bloom_enabled {
            if bloom.is_none() {
                // Add Bloom component if not present
                commands.entity(entity).insert(Bloom {
                    intensity: post_process_settings.bloom_intensity,
                    ..Bloom::NATURAL
                });
                info!("[PostProcess] Bloom enabled on camera");
            }
        } else {
            if bloom.is_some() {
                // Remove Bloom component if present
                commands.entity(entity).remove::<Bloom>();
                info!("[PostProcess] Bloom disabled on camera");
            }
        }

        // Handle SSAO: remove the component when disabled so the SSAO pass is
        // skipped. Previously this only downgraded to Low (still full cost).
        if post_process_settings.ssao_enabled {
            if ssao.is_none() {
                commands.entity(entity).insert(ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                    ..Default::default()
                });
                info!("[PostProcess] SSAO enabled on camera");
            }
        } else if ssao.is_some() {
            commands.entity(entity).remove::<ScreenSpaceAmbientOcclusion>();
            info!("[PostProcess] SSAO disabled on camera");
        }

        // Handle Volumetric Fog: remove when disabled so raymarching is skipped.
        // Previously step_count=1 still dispatched the volume.
        if post_process_settings.volumetric_fog_enabled {
            if volumetric_fog.is_none() {
                commands.entity(entity).insert(VolumetricFog {
                    ambient_intensity: 0.1,
                    step_count: 64,
                    ..Default::default()
                });
                info!("[PostProcess] Volumetric fog enabled on camera");
            }
        } else if volumetric_fog.is_some() {
            commands.entity(entity).remove::<VolumetricFog>();
            info!("[PostProcess] Volumetric fog disabled on camera");
        }
    }
}

/// System to apply water settings from the resource to water materials
/// This allows live adjustment of water parameters via the Settings UI
/// Also syncs fog parameters from ZoneLighting to integrate water with scene fog
fn apply_water_settings(
    water_settings: Res<WaterSettings>,
    zone_lighting: Res<render::ZoneLighting>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
) {
    use bevy::ecs::change_detection::DetectChanges;

    // Update if water settings or zone lighting have changed
    if water_settings.is_changed() || zone_lighting.is_changed() {
        for (_, material) in water_materials.iter_mut() {
            material.settings = water_settings.clone();
            // Sync fog parameters from ZoneLighting for water-scene integration
            material.fog_color = zone_lighting.fog_color.extend(1.0);
            material.fog_density = zone_lighting.fog_density;
            material.fog_min_density = zone_lighting.fog_min_density;
            material.fog_max_density = zone_lighting.fog_max_density;
        }
    }
}

/// System to spawn starry sky sphere and moon directional light
/// Creates a large inverted sphere with procedural star material and a moon light source
fn spawn_starry_sky_and_moon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StarrySkyMaterial>>,
    starry_sky_settings: Res<StarrySkySettings>,
) {
    use bevy::math::primitives::Sphere;
    use bevy_light::DirectionalLight as DirectionalLightComponent;

    // Sky sphere follows the camera (see follow_sky_to_camera), so a modest radius
    // is enough and keeps depth precision + far plane small. Previously 50000 forced
    // far=100000. Shader uses view-relative direction, so centering on camera is correct.
    let sky_sphere_radius = 4000.0;

    log::info!(
        "[STARRY SKY] Spawning sky sphere (radius {}), star_density: {}, star_brightness: {}, night_factor: {}",
        sky_sphere_radius,
        starry_sky_settings.star_density,
        starry_sky_settings.star_brightness,
        starry_sky_settings.night_factor
    );

    // Create starry sky sphere mesh (large sphere centered at world origin)
    let sphere = Sphere::new(sky_sphere_radius);
    let mut sky_mesh = Mesh::from(sphere);

    // Flip normals for inside rendering (we're inside the sphere looking out)
    if let Some(normals) = sky_mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        if let VertexAttributeValues::Float32x3(normals) = normals {
            for normal in normals.iter_mut() {
                normal[0] = -normal[0];
                normal[1] = -normal[1];
                normal[2] = -normal[2];
            }
        } else {
            log::warn!("[STARRY SKY] Normals attribute has unexpected format!");
        }
    } else {
        log::warn!("[STARRY SKY] No normals attribute found in mesh!");
    }

    // CRITICAL FIX: Reverse the winding order of triangles for inside rendering
    // When viewing a sphere from inside, the triangles are front-facing if we reverse the indices
    // Without this, backface culling removes all triangles and the sky is invisible
    if let Some(indices) = sky_mesh.indices_mut() {
        match indices {
            Indices::U32(indices) => {
                // Reverse each triangle (swap v1 and v2 of each triangle)
                for chunk in indices.chunks_mut(3) {
                    chunk.swap(1, 2);
                }
            }
            Indices::U16(indices) => {
                for chunk in indices.chunks_mut(3) {
                    chunk.swap(1, 2);
                }
            }
        }
    } else {
        log::warn!("[STARRY SKY] No indices in mesh - mesh may use non-indexed rendering");
    }

    // Create material with current settings
    let sky_material = StarrySkyMaterial {
        time: 0.0,
        star_density: starry_sky_settings.star_density,
        star_brightness: starry_sky_settings.star_brightness,
        night_factor: starry_sky_settings.night_factor,
        moon_phase: starry_sky_settings.moon_phase,
        moon_direction: starry_sky_settings.moon_direction,
    };

    // Spawn starry sky entity
    let sky_mesh_handle = meshes.add(sky_mesh);
    let sky_material_handle = materials.add(sky_material);

    let sky_entity = commands
        .spawn((
            StarrySky,
            Mesh3d(sky_mesh_handle),
            MeshMaterial3d(sky_material_handle),
            Transform::from_xyz(0.0, 0.0, 0.0), // Center of world - sphere is large enough to contain camera
            Visibility::Visible,
            bevy::camera::visibility::NoFrustumCulling, // CRITICAL: Prevent frustum culling of sky sphere
        ))
        .id();

    log::info!("[STARRY SKY] StarrySky entity spawned with id: {:?}", sky_entity);

    // Spawn moon directional light (separate from sun).
    // Night illumination; shadows stay OFF in all states per
    // update_shadows_for_time_of_day_system (second shadow map doubles cost).
    // Illuminance is modulated by time-of-day (0 by day, up to 3000 at night).
    let moon_entity = commands
        .spawn((
            MoonLight,
            DirectionalLightComponent {
                illuminance: 5000.0,                 // Moonlight intensity (much dimmer than sun)
                color: Color::srgb(0.8, 0.85, 0.95), // Slightly blue-white moonlight
                shadow_maps_enabled: false,
                // No contact shadows for the moon (second shadow map doubles cost).
                contact_shadows_enabled: false,
                shadow_depth_bias: 0.02,
                shadow_normal_bias: 1.0,
                affects_lightmapped_mesh_diffuse: true,
            },
            Transform::from_xyz(0.0, 100.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            Visibility::Visible,
        ))
        .id();

    log::info!("[STARRY SKY] MoonLight entity spawned with id: {:?}", moon_entity);
}

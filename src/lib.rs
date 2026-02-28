#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(warnings)]
use log::{info, warn, error};
use animation::RoseAnimationPlugin;
use bevy::{
        asset::AssetApp,
        core_pipeline::bloom::Bloom,
        core_pipeline::dof::{DepthOfField, DepthOfFieldMode},
        core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass},
        core_pipeline::smaa::Smaa,
        pbr::{Atmosphere, AtmosphereSettings, ExtendedMaterial, MaterialPlugin, StandardMaterial, MeshMaterial3d, VolumetricFog, VolumetricLight, FogVolume, ShadowFilteringMethod, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel},
        render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
        render::experimental::occlusion_culling::OcclusionCulling,
        prelude::{
            apply_deferred, default, in_state, not, resource_exists, App, AppExtStates, AssetServer, Assets, Camera, Camera3d,
            ClearColorConfig, Color, Commands, Cuboid, Entity, Handle, Image, InheritedVisibility, IntoScheduleConfigs,
            Local, Mesh, Mesh3d, Msaa, OnEnter, OnExit, PerspectiveProjection,
            PluginGroup, PostStartup, PostUpdate, PreUpdate, Projection, Quat, Query, Res, ResMut, Startup, State,
            SystemSet, Time, Transform, Update, Vec3, ViewVisibility, Visibility, With, Without, World,
        },
        render::view::VisibilitySystems,
        render::camera::Exposure,
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        render_asset::RenderAssets,
        ExtractSchedule, Render, RenderApp,
    },
        transform::{TransformSystem, components::GlobalTransform},
        window::{Window, WindowMode},
    };
use bevy_egui::{egui, input::egui_wants_any_pointer_input, EguiContext, EguiContexts, EguiRenderOutput};
use bevy_rapier3d::plugin::PhysicsSet;
use enum_map::enum_map;
use exe_resource_loader::{ExeResourceCursor, ExeResourceLoader};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
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
pub mod diagnostics;
pub mod effect_loader;
pub mod events;
pub mod exe_resource_loader;
pub mod map_editor;
pub mod model_loader;
pub mod protocol;
pub mod render;
pub use render::DamageDigitMaterial;
pub mod resources;
pub mod scripting;
pub mod systems;
pub mod ui;
pub mod dds_image_loader;
pub mod vfs_asset_io;
pub mod zms_asset_loader;
pub mod zone_loader;
pub mod blood_effect_plugin;

use audio::OddioPlugin;
use diagnostics::RenderDiagnosticsPlugin;
use events::{
    BankEvent, CharacterSelectEvent, ChatBubbleEvent, ChatboxEvent, ClanDialogEvent, ClientEntityEvent,
    ConversationDialogEvent, FlightToggleEvent, GameConnectionEvent, HitEvent, LoadZoneEvent, LoginEvent,
    MessageBoxEvent, MoveDestinationEffectEvent, MoveSpeedSetEvent, NetworkEvent, NpcStoreEvent,
    NumberInputDialogEvent, PartyEvent, PersonalStoreEvent, PlayerCommandEvent, QuestTriggerEvent,
    SpawnEffectEvent, SpawnProjectileEvent, SystemFuncEvent, UseItemEvent, WorldConnectionEvent,
    ZoneEvent, ZoneLoadedFromVfsEvent,
    };
use model_loader::ModelLoader;
use render::{
    RoseRenderPlugin,
    DamageDigitMaterialPlugin,
    ParticleMaterialPlugin,
    RoseObjectExtension,
    RoseTerrainExtension,
    RoseWaterExtension,
    RoseEffectExtension,
    TrailEffectRenderPlugin,
    WorldUiRenderPlugin,
    ZoneLightingPlugin,
    ExtensionMaterialPlugin,
    RoseObjectMaterialPlugin,
    WaterMaterial,
    debug_particle_rendering,
    particle_performance_monitor,
    UnderwaterEffectPlugin,
    UnderwaterSettings,
    CameraUnderwaterState,
    StarrySkyMaterialPlugin,
    StarrySkySettings,
    StarrySky,
    MoonLight,
    StarrySkyMaterial,
    create_starry_sky_mesh,
    update_starry_sky_system,
    update_starry_sky_night_factor,
    toggle_atmosphere_based_on_time,
    sky_sphere_follow_camera_system,
    AtmosphereState,
};
use resources::{
    load_ui_resources, run_network_thread, ui_requested_cursor_apply_system, update_ui_resources,
    AppState, ClientEntityList, CurrentZone, DamageDigitsSpawner, DebugRenderConfig, FlightSettings, GameData, LoginCameraAnimation, MonsterChatterPhrases, NameTagSettings,
    NetworkThread, NetworkThreadMessage, RenderConfiguration, RenderExtractionDiagnostics, SelectedTarget, ServerConfiguration,
    SoundCache, SoundSettings, SpecularTexture, VfsResource, WaterSettings, WorldTime, ZoneTime,
};
use scripting::RoseScriptingPlugin;
use systems::{
    ability_values_system, animation_effect_system, animation_sound_system, auto_login_system,
    background_music_system, character_model_add_collider_system, character_model_blink_system,
    character_model_update_system, character_select_enter_system, character_select_event_system,
    character_select_exit_system, character_select_input_system,
    character_select_models_system, character_select_system, CharacterSelectInputState,
    chat_bubble_spawn_system, chat_bubble_update_system, chat_bubble_cleanup_system, chat_bubble_orphan_cleanup_system,
    add_monster_chatter_system, monster_chatter_system,
    clan_system, client_entity_event_system, collision_height_only_system,
    collision_player_system, collision_player_system_join_zone, command_system,
    conversation_dialog_system, cooldown_system, damage_digit_render_system,
    create_damage_digit_material_system,
    directional_light_system, effect_system, facing_direction_system,
    flight_movement_system, flight_pose_system, flight_pose_blend_update_system, flight_toggle_system, ensure_flight_state_system,
    free_camera_system, game_connection_system, game_mouse_input_system, game_state_enter_system,
    game_zone_change_system, hit_event_system, item_drop_model_add_collider_system,
    item_drop_model_system, login_connection_system, login_event_system, login_state_enter_system,
    login_state_exit_system, login_system, model_viewer_enter_system, model_viewer_exit_system,
    model_viewer_system, move_destination_effect_system, move_speed_set_system, name_tag_system,
    name_tag_update_color_system, name_tag_update_healthbar_system, name_tag_visibility_system,
    network_thread_system, npc_idle_sound_system, npc_model_add_collider_system,
    npc_model_update_system, orbit_camera_system, particle_sequence_system,
    particle_storage_buffer_update_system, create_default_particle_texture,
    passive_recovery_system, pending_damage_system, pending_skill_effect_system,
    personal_store_model_add_collider_system, personal_store_model_system, player_command_system,
    projectile_system, quest_trigger_system, spawn_effect_system, spawn_projectile_system,
    status_effect_system, system_func_event_system, update_position_system, use_item_event_system,
    vehicle_model_system, vehicle_sound_system, visible_status_effects_system,
    world_connection_system, world_time_system, zone_time_system, zone_viewer_enter_system,
    // DISABLED: color_grading_time_of_day_system conflicts with Bevy 0.16 Atmosphere
    // color_grading_time_of_day_system,
    DebugInspectorPlugin, FishPlugin, BirdPlugin, DirtDashPlugin, WingSpawnPlugin, WindEffectPlugin,
};
use ui::{
    load_dialog_sprites_system, ui_bank_system, ui_character_create_system,
    ui_character_info_system, ui_character_select_name_tag_system, ui_character_select_system,
    ui_chatbox_system, ui_clan_system, ui_create_clan_system, ui_debug_camera_info_system,
    ui_debug_client_entity_list_system, ui_debug_command_viewer_system,
    ui_debug_diagnostics_system, ui_debug_dialog_list_system, ui_debug_effect_list_system,
    ui_debug_entity_inspector_system, ui_debug_item_list_system, ui_debug_menu_system,
    ui_debug_npc_list_system, ui_debug_physics_system, ui_debug_render_system,
    ui_debug_skill_list_system, ui_debug_zone_lighting_system, ui_debug_zone_list_system,
    ui_debug_zone_time_system, ui_drag_and_drop_system, ui_game_menu_system, ui_hotbar_system,
    ui_inventory_system, ui_item_drop_name_system, ui_login_system, ui_message_box_system,
    ui_minimap_system, ui_npc_store_system, ui_number_input_dialog_system, ui_party_option_system,
    ui_party_system, ui_personal_store_system, ui_player_info_system, ui_quest_list_system,
    ui_respawn_system, ui_selected_target_system, ui_server_select_system, ui_settings_system,
    ui_skill_list_system, ui_skill_tree_system, ui_sound_event_system, ui_status_effects_system,
    ui_window_sound_system, widgets::Dialog, DepthOfFieldSettings, DialogLoader, UiSoundEvent, UiStateDebugWindows,
    UiStateDragAndDrop, UiStateWindows,
};
use dds_image_loader::DdsImageLoader;
use vfs_asset_io::{VfsAssetIo, VfsAssetReaderPlugin};
use zms_asset_loader::{ZmsAssetLoader, ZmsMaterialNumFaces, ZmsNoSkinAssetLoader};
use zone_loader::{zone_loader_system, zone_loaded_from_vfs_system, force_zone_visibility_system, ZoneLoader, ZoneLoaderAsset, ZoneLoadChannelReceiver, ZoneLoadChannelSender, MemoryTrackingResource};

use crate::components::{CollisionPlayer, SoundCategory, Zone, VegetationSwayPlugin};

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
                    // Get the parent directory of the VFS index file
                    // For relative paths like "data.idx", parent() returns empty string
                    // In that case, use the current directory
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|p| if p.as_os_str().is_empty() {
                            std::env::current_dir().unwrap_or_default()
                        } else {
                            p.to_path_buf()
                        })
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

                    log::info!("Loading game data from AruaVfs {}", path);
                    vfs_devices.push(Box::new(
                        AruaVfsIndex::load(Path::new(path), &index_root_path.join("data.rose"))
                            .unwrap_or_else(|_| panic!("Failed to load AruaVfs at {}", path)),
                    ));

                    log::info!(
                        "Loading game data from AruaVfs root path {}",
                        index_root_path.to_string_lossy()
                    );
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path.clone())));
                    // Use the VFS root path as base path for saving
                    if base_path.is_none() {
                        base_path = Some(index_root_path);
                    }
                }
                FilesystemDeviceConfig::TitanVfs(path) => {
                    // Get the parent directory of the VFS index file
                    // For relative paths like "data.idx", parent() returns empty string
                    // In that case, use the current directory
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|p| if p.as_os_str().is_empty() {
                            std::env::current_dir().unwrap_or_default()
                        } else {
                            p.to_path_buf()
                        })
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

                    log::info!("Loading game data from TitanVfs {}", path);
                    vfs_devices.push(Box::new(
                        TitanVfsIndex::load(Path::new(path), &index_root_path.join("data.trf"))
                            .unwrap_or_else(|_| panic!("Failed to load TitanVfs at {}", path)),
                    ));

                    log::info!("Loading game data from TitanVfs root path {}", index_root_path.to_string_lossy());
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path.clone())));
                    // Use the VFS root path as base path for saving
                    if base_path.is_none() {
                        base_path = Some(index_root_path);
                    }
                }
                FilesystemDeviceConfig::Vfs(path) => {
                    log::info!("Loading game data from Vfs {}", path);
                    vfs_devices.push(Box::new(
                        VfsIndex::load(Path::new(path))
                            .unwrap_or_else(|_| panic!("Failed to load Vfs at {}", path)),
                    ));

                    // Get the parent directory of the VFS index file
                    // For relative paths like "data.idx", parent() returns empty string
                    // In that case, use the current directory
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|p| if p.as_os_str().is_empty() {
                            std::env::current_dir().unwrap_or_default()
                        } else {
                            p.to_path_buf()
                        })
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                    
                    log::info!("Loading game data from Vfs root path {}", index_root_path.to_string_lossy());
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path.clone())));
                    // Use the VFS root path as base path for saving
                    if base_path.is_none() {
                        base_path = Some(index_root_path);
                    }
                }
                FilesystemDeviceConfig::IrosePh(path) => {
                    // Get the parent directory of the VFS index file
                    // For relative paths like "data.idx", parent() returns empty string
                    // In that case, use the current directory
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|p| if p.as_os_str().is_empty() {
                            std::env::current_dir().unwrap_or_default()
                        } else {
                            p.to_path_buf()
                        })
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

                    log::info!("Loading game data from iRosePH {}", path);
                    vfs_devices.push(Box::new(
                        IrosePhVfsIndex::load(Path::new(path))
                            .unwrap_or_else(|_| panic!("Failed to load iRosePH VFS at {}", path)),
                    ));

                    log::info!(
                        "Loading game data from iRosePH root path {}",
                        index_root_path.to_string_lossy()
                    );
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path.clone())));
                    // Use the VFS root path as base path for saving
                    if base_path.is_none() {
                        base_path = Some(index_root_path);
                    }
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
                app.world_mut()
                    .send_event(LoadZoneEvent::new(
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
                app.world_mut()
                    .send_event(LoadZoneEvent::new(
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum GameSystemOrdering {
    CommandSystem,
    PositionSystem,
    CollisionPlayerSystemJoinZone,
    CollisionPlayerSystem,
    CollisionHeightOnlySystem,
    CooldownSystem,
    ClientEntityEventSystem,
    UseItemEventSystem,
    GameMouseInputSystem,
    PlayerCommandSystem,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum GameStateSystemSets {
    GameUi,
    PlayerCommand,
    GameSystems,
    GameUiSystems,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum ModelViewerSystemSets {
    ModelViewer,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum LoginSystemOrdering {
    LoginSystem,
    LoginEventSystem,
    UiLogin,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum CharacterSelectSystemOrdering {
    CharacterSelectSystem,
    CharacterSelectInputSystem,
    CharacterSelectEventSystem,
    CharacterSelectModelsSystem,
    UiCharacterSelect,
    /// System set for all character select systems - used to apply run_if to the group
    AllCharacterSelectSystems,
}

fn run_client(config: &Config, app_state: AppState, mut systems_config: SystemsConfig) {
    println!("run_client() function entered");
    log::info!("[VFS INIT] Starting VFS initialization...");
    log::info!("[VFS INIT] Config has {} filesystem devices", config.filesystem.devices.len());
    
    let (virtual_filesystem, base_path) =
        if let Some((vfs, base)) = config.filesystem.create_virtual_filesystem() {
            log::info!("[VFS INIT] VFS created successfully!");
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

    log::info!("[VFS DIAGNOSTIC] Creating VfsAssetReaderPlugin");
    // OPTIMIZATION: Only clone once for VfsResource. VfsAssetReaderPlugin retrieves
    // the VFS from VfsResource during build, eliminating a redundant Arc clone.
    // Previously: 2 clones (one for plugin, one for resource)
    // Now: 1 clone (only for resource, plugin retrieves from resource)
    app.insert_resource(VfsResource {
        vfs: virtual_filesystem.clone(),
        base_path,
    })
    // Register VFS asset reader BEFORE DefaultPlugins (required by Bevy 0.13)
    // VfsAssetReaderPlugin gets the VFS from VfsResource instead of holding its own Arc
    .add_plugins(VfsAssetReaderPlugin::new());
    log::info!("[VFS DIAGNOSTIC] VfsAssetReaderPlugin added to app");

    // DIAGNOSTIC: Verify VFS contains required files
    app.add_systems(bevy::app::Startup, |vfs_resource: Res<VfsResource>| {
        log::info!("[VFS DIAGNOSTIC] Checking if VFS contains required files...");

        let test_paths = vec![
            "3DDATA/CONTROL/XML/DLGAVATARSTORE.XML",
            "3DDATA/CONTROL/XML/DELIVERYSTORE.XML",
            "3DDATA/CONTROL/XML/DLGCHAT.XML",
            "3DDATA/CONTROL/XML/DLGADDFRIEND.XML",
            "3DDATA/CONTROL/XML/DLGBANK.XML",
            "3DDATA/CONTROL/XML/UI_STRID.ID",
            "4.zone_loader",
        ];

        // for path in test_paths {
        //     match vfs_resource.vfs.open_file(path) {
        //         Ok(_) => log::info!("[VFS DIAGNOSTIC] VFS contains file: {}", path),
        //         Err(_) => log::warn!("[VFS DIAGNOSTIC] VFS does NOT contain file: {}", path),
        //     }
        // }
    });

    // DIAGNOSTIC: Log asset server configuration
    app.add_systems(bevy::app::Startup, |asset_server: Res<bevy::asset::AssetServer>| {
        log::info!("[ASSET SERVER DIAGNOSTIC] Asset server initialized");

        // Try to get the default asset source to see what reader is being used
        match asset_server.get_source(bevy::asset::io::AssetSourceId::Default) {
            Ok(source) => {
                log::info!("[ASSET SERVER DIAGNOSTIC] Default asset source found");
                // Log the type of reader being used
                let reader = source.reader();
                let reader_type = std::any::type_name_of_val(reader);
                log::info!("[ASSET SERVER DIAGNOSTIC] Default asset reader type: {}", reader_type);
            }
            Err(e) => {
                log::error!("[ASSET SERVER DIAGNOSTIC] Failed to get default asset source: {:?}", e);
            }
        }

        // DIAGNOSTIC: Log registered asset loaders
        log::info!("[ASSET SERVER DIAGNOSTIC] Checking registered asset loaders...");
        // Note: Bevy doesn't provide a direct way to list all registered loaders,
        // but we can infer from the extensions we know about
        log::info!("[ASSET SERVER DIAGNOSTIC] Known asset extensions:");
        log::info!("[ASSET SERVER DIAGNOSTIC]   - .zone_loader (ZoneLoader)");
        log::info!("[ASSET SERVER DIAGNOSTIC]   - .zms (ZmsAssetLoader)");
        log::info!("[ASSET SERVER DIAGNOSTIC]   - .zmo (ZmoAssetLoader)");
        log::info!("[ASSET SERVER DIAGNOSTIC]   - .exe (ExeResourceLoader)");
        log::info!("[ASSET SERVER DIAGNOSTIC]   - .dialog (DialogLoader)");
    });

    // Initialise bevy engine
    app.add_plugins((
            bevy::prelude::DefaultPlugins
                .set(bevy::render::RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::all()),
                        ..Default::default()
                    }),
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
                            window_width,
                            window_height,
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
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::FrameTimeDiagnosticsPlugin::new(60),  // 60 frame history
        ));

    // Initialise 3rd party bevy plugins
    // Note: RapierConfiguration is no longer a Resource in Bevy 0.15
    // Configuration is now handled through the RapierPhysicsPlugin
    app.add_plugins(bevy_egui::EguiPlugin { enable_multipass_for_primary_context: false });
    app.add_plugins(bevy_rapier3d::prelude::RapierPhysicsPlugin::<bevy_rapier3d::prelude::NoUserData>::default());
    // Disabled: RapierDebugRenderPlugin (debug plugin)
    // Disabled: RenderDocPlugin (debug plugin)
    app.add_plugins(OddioPlugin);

    // Initialise rose stuff
    log::info!("[ASSET LOADER DIAGNOSTIC] Registering asset loaders...");
    log::info!("[ASSET LOADER DIAGNOSTIC] Registering ZmsAssetLoader");

    // Create channel for async zone loading
    let (tx, rx) = mpsc::channel();
    app.insert_resource(ZoneLoadChannelSender(tx));
    app.insert_resource(ZoneLoadChannelReceiver(std::sync::Mutex::new(rx)));
    log::info!("[ZONE LOADER] Channel for async zone loading created and registered");

    // Initialize memory tracking resource for zone loading
    app.init_resource::<MemoryTrackingResource>();
    log::info!("[ZONE LOADER] MemoryTrackingResource initialized");

    app.init_resource::<RenderExtractionDiagnostics>();
    log::info!("[ZONE LOADER] Debug diagnostics resources initialized");

    app.register_asset_loader(ZmsAssetLoader)
        .init_asset::<ZmsMaterialNumFaces>()
        .register_asset_loader(ZmsNoSkinAssetLoader)
        .register_asset_loader(DdsImageLoader)
        .register_asset_loader(ExeResourceLoader)
        .init_asset::<ExeResourceCursor>()
        .register_asset_loader(DialogLoader)
        .init_asset::<Dialog>()
        .register_asset_loader(zone_loader::ZoneLoader)
        .init_asset::<zone_loader::ZoneLoaderAsset>()
        .insert_resource(RenderConfiguration {
            passthrough_terrain_textures: config.graphics.passthrough_terrain_textures,
            trail_effect_duration_multiplier: config.graphics.trail_effect_duration_multiplier,
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
            DamageDigitMaterialPlugin,        // ← Fixes the immediate panic
            ParticleMaterialPlugin,

            // ExtendedMaterial plugins for object, terrain, water, and effect mesh
            // Use custom RoseObjectMaterialPlugin which includes zone lighting support
            RoseObjectMaterialPlugin::default(),
        ));
    log::info!("[MATERIAL PLUGIN] RoseObjectExtension plugin registered successfully");

    app.add_plugins((
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, RoseTerrainExtension>>::default(),
        ));
    log::info!("[MATERIAL PLUGIN] RoseTerrainExtension plugin registered successfully");

    app.add_plugins((
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, RoseWaterExtension>>::default(),
        ));
    log::info!("[MATERIAL PLUGIN] RoseWaterExtension plugin registered successfully");

    app.add_plugins((
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>::default(),
        ));
    log::info!("[MATERIAL PLUGIN] RoseEffectExtension plugin registered successfully");

    // Register extension material shaders
    app.add_plugins(ExtensionMaterialPlugin);
    log::info!("[MATERIAL PLUGIN] ExtensionMaterialPlugin registered successfully");

    // Optional: Add these for full rendering support
    app.add_plugins((
            TrailEffectRenderPlugin,
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
            
            // OPTIONAL: RenderDiagnosticsPlugin is for debugging rendering issues - keep disabled to reduce log noise
            // RenderDiagnosticsPlugin,

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

            // Underwater rendering effect
            UnderwaterEffectPlugin,

            // Procedural starry sky with moon lighting
            StarrySkyMaterialPlugin,

            // Blood effect system (spatter decals, gash wounds)
            blood_effect_plugin::BloodEffectPlugin,

            // Map editor system
            map_editor::MapEditorPlugin,
        ));
    log::info!("[ASSET LOADER DIAGNOSTIC] Asset loaders registered successfully");

    // Material Plugin Diagnostic Logging
    log::info!("[MATERIAL PLUGIN] DamageDigitMaterialPlugin registered");
    log::info!("[MATERIAL PLUGIN] ParticleMaterialPlugin registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseObjectExtension> registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseTerrainExtension> registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseWaterExtension> registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseEffectExtension> registered");

    // Setup state
    app.insert_state(app_state);

    app.add_event::<BankEvent>()
        .add_event::<ChatBubbleEvent>()
        .add_event::<ChatboxEvent>()
        .add_event::<CharacterSelectEvent>()
        .add_event::<ClanDialogEvent>()
        .add_event::<ClientEntityEvent>()
        .add_event::<ConversationDialogEvent>()
        .add_event::<FlightToggleEvent>()
        .add_event::<GameConnectionEvent>()
        .add_event::<HitEvent>()
        .add_event::<LoginEvent>()
        .add_event::<LoadZoneEvent>()
        .add_event::<MessageBoxEvent>()
        .add_event::<MoveDestinationEffectEvent>()
        .add_event::<MoveSpeedSetEvent>()
        .add_event::<NetworkEvent>()
        .add_event::<NumberInputDialogEvent>()
        .add_event::<NpcStoreEvent>()
        .add_event::<PartyEvent>()
        .add_event::<PersonalStoreEvent>()
        .add_event::<PlayerCommandEvent>()
        .add_event::<QuestTriggerEvent>()
        .add_event::<SystemFuncEvent>()
        .add_event::<SpawnEffectEvent>()
        .add_event::<SpawnProjectileEvent>()
        .add_event::<UseItemEvent>()
        .add_event::<WorldConnectionEvent>()
        .add_event::<ZoneEvent>()
        .add_event::<ZoneLoadedFromVfsEvent>()
        .add_event::<UiSoundEvent>();

    app.add_systems(
        PostUpdate,
        apply_deferred,
    );

    app.add_systems(
        PostUpdate,
        (apply_deferred,).in_set(GameStages::DebugRenderPreFlush),
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
            ModelSystemSets::PersonalStoreModelAddCollider.after(ModelSystemSets::PersonalStoreModel),
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
            EffectSystemSets::PendingDamage.after(EffectSystemSets::AnimationEffect).after(EffectSystemSets::Projectile),
            EffectSystemSets::PendingSkillEffect.after(EffectSystemSets::AnimationEffect).after(EffectSystemSets::Projectile),
            EffectSystemSets::HitEvent.after(EffectSystemSets::AnimationEffect).after(EffectSystemSets::PendingSkillEffect).after(EffectSystemSets::Projectile),
            EffectSystemSets::SpawnEffect.after(EffectSystemSets::AnimationEffect).after(EffectSystemSets::HitEvent),
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
    // name_tag_system uses EguiContexts
    app.add_systems(
        Update,
        name_tag_system.after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );
    // chat_bubble_spawn_system uses EguiContexts for text rendering
    app.add_systems(
        Update,
        chat_bubble_spawn_system.after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );
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
    app.add_systems(
        Update,
        (
            add_monster_chatter_system,
            monster_chatter_system,
        ),
    );
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
            spawn_effect_system,
            visible_status_effects_system,
            move_destination_effect_system,
            create_damage_digit_material_system.before(damage_digit_render_system),
            damage_digit_render_system,
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
            // Starry sky material update - updates uniforms for twinkling and night factor
            // Runs after update_starry_sky_night_factor to use updated night_factor value
            update_starry_sky_system.after(update_starry_sky_night_factor),
        ),
    );
    // update_ui_resources uses EguiContexts
    app.add_systems(Update, update_ui_resources.after(bevy_egui::EguiPreUpdateSet::InitContexts));

    // ui_requested_cursor_apply_system uses EguiContexts
    app.add_systems(PostUpdate, ui_requested_cursor_apply_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));

    app.add_systems(
        Update,
        ui_item_drop_name_system.after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );

    app.add_systems(
        Update,
        (ui_message_box_system, ui_number_input_dialog_system)
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );
    app.add_systems(
        Update,
        (
            ui_window_sound_system,
            ui_sound_event_system,
        )
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );

    app.add_systems(
        Update,
        ui_debug_menu_system
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );


    app.add_systems(
        Update,
        (
            ui_debug_camera_info_system,
            ui_debug_client_entity_list_system,
            ui_debug_command_viewer_system,
            ui_debug_dialog_list_system,
            ui_debug_effect_list_system,
            ui_debug_entity_inspector_system,
            ui_debug_item_list_system,
            ui_debug_npc_list_system,
        )
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );

    // DISABLED: app.add_systems(Update, ui_debug_physics_system); // Too many parameters for Bevy 0.15
    app.add_systems(Update, ui_debug_render_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_skill_list_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_zone_lighting_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_zone_list_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_zone_time_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));
    // DISABLED: app.add_systems(Update, ui_debug_diagnostics_system);

    // character_model_blink_system in PostUpdate to avoid any conflicts with model destruction
    // e.g. through the character select exit system.
    app.add_systems(PostUpdate, character_model_blink_system);

    // Sky sphere follows camera in PostUpdate to ensure camera transform is up to date
    app.add_systems(PostUpdate, sky_sphere_follow_camera_system.after(TransformSystem::TransformPropagate));

    // vehicle_model_system in after ::Update but before ::PostUpdate to avoid any conflicts,
    // with model destruction but to also be before global transform is calculated.
    app.add_systems(
        PostUpdate,
        (
            vehicle_model_system,
            vehicle_sound_system,
        ),
    );

    // Configure vehicle system ordering
    app.configure_sets(
        PostUpdate,
        GameStages::AfterUpdate,
    );
    app.add_systems(
        PostUpdate,
        vehicle_sound_system,
    );

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
        )
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
        )
    );

    // Zone Viewer
    app.add_systems(OnEnter(AppState::ZoneViewer), zone_viewer_enter_system);

    // Map Editor
    app.add_systems(OnEnter(AppState::MapEditor), map_editor::map_editor_enter_system);
    app.add_systems(OnExit(AppState::MapEditor), map_editor::map_editor_exit_system);

    // CRITICAL DIAGNOSTIC: Check if transform and visibility propagation sets are running
    app.add_systems(
        PostUpdate,
        (
            |mut frame_count: Local<u32>| {
                *frame_count += 1;
                if *frame_count % 60 == 0 {
                    // info!("[SCHEDULE CHECK] TransformPropagate set is running");
                }
            }
        ).in_set(TransformSystem::TransformPropagate)
    );
    app.add_systems(
        PostUpdate,
        (
            |mut frame_count: Local<u32>| {
                *frame_count += 1;
                if *frame_count % 60 == 0 {
                    // info!("[SCHEDULE CHECK] VisibilityPropagate set is running");
                }
            }
        ).in_set(VisibilitySystems::VisibilityPropagate)
    );
    app.add_systems(
        PostUpdate,
        (
            |mut frame_count: Local<u32>| {
                *frame_count += 1;
                if *frame_count % 60 == 0 {
                    // info!("[SCHEDULE CHECK] CheckVisibility set is running");
                }
            }
        ).in_set(VisibilitySystems::CheckVisibility)
    );
    app.add_systems(
        PostUpdate,
        (
            |mut frame_count: Local<u32>| {
                *frame_count += 1;
                if *frame_count % 60 == 0 {
                    // info!("[SCHEDULE CHECK] CalculateBounds set is running");
                }
            }
        ).in_set(VisibilitySystems::CalculateBounds)
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

    app.add_systems(
        Update,
        (
            login_system.before(login_event_system),
            login_event_system,
        )
        .run_if(in_state(AppState::GameLogin))
        .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );

    app.add_systems(
        Update,
        (ui_login_system, ui_server_select_system)
            .run_if(in_state(AppState::GameLogin))
            .in_set(UiSystemSets::Ui)
            .after(login_system)
            .before(login_event_system)
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
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
    // character_select_system uses EguiContexts for UI dialogs
    app.add_systems(
        Update,
        character_select_system
            .run_if(in_state(AppState::GameCharacterSelect))
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
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

    // UI systems
    app.add_systems(
        Update,
        (
            ui_character_create_system,
            ui_character_select_system,
            ui_character_select_name_tag_system,
        )
            .run_if(in_state(AppState::GameCharacterSelect))
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
    );
    // character_select_input_system uses EguiContexts to check if egui wants pointer input
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
        .init_resource::<ClientEntityList>()
        .init_resource::<DebugRenderConfig>()
        .init_resource::<WorldTime>()
        .init_resource::<ZoneTime>()
        .init_resource::<SelectedTarget>()
        .init_resource::<NameTagSettings>()
        .init_resource::<DepthOfFieldSettings>()
        .init_resource::<WaterSettings>()
        .init_resource::<FlightSettings>()
        .init_resource::<MonsterChatterPhrases>()
        .init_resource::<AtmosphereState>();

    app.add_systems(OnEnter(AppState::Game), game_state_enter_system);

    // Spawn starry sky and moon light entities on startup
    app.add_systems(PostStartup, spawn_starry_sky_and_moon);

    // System to apply depth of field settings from the resource to the camera
    app.add_systems(Update, apply_depth_of_field_settings);
    
    // System to apply water settings from the resource to water materials
    app.add_systems(Update, apply_water_settings);

    // Register systems individually to avoid Bevy 0.13's IntoSystemConfigs trait bound issues
    // Game systems - part 1
    app.add_systems(Update, ability_values_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, clan_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, command_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, facing_direction_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, update_position_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, collision_height_only_system.run_if(in_state(AppState::Game)));
    // CRITICAL: collision_player_system_join_zone must run BEFORE collision_player_system
    // - join_zone uses a long raycast (Y=100000) to find initial ground height on spawn
    // - collision_player_system uses short raycast for continuous terrain following
    // Using Added<CollisionPlayer> filter ensures join_zone only runs once on spawn
    app.add_systems(
        Update,
        collision_player_system_join_zone
            .run_if(in_state(AppState::Game))
            .before(collision_player_system),
    );
    app.add_systems(Update, collision_player_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, cooldown_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, client_entity_event_system.run_if(in_state(AppState::Game)));
    
    // Flight systems - ensure_flight_state_system runs before flight_toggle_system
    app.add_systems(Update, ensure_flight_state_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, flight_toggle_system.run_if(in_state(AppState::Game)).after(ensure_flight_state_system));
    app.add_systems(Update, flight_movement_system.run_if(in_state(AppState::Game)).after(flight_toggle_system));
    // Flight pose blend update system - updates pose_blend value on FlightState
    app.add_systems(Update, flight_pose_blend_update_system.run_if(in_state(AppState::Game)).after(flight_toggle_system));
    // Flight pose system applies visual-only rotations to character model parts
    // Runs after facing_direction_system and character_model_update_system
    app.add_systems(Update, flight_pose_system.run_if(in_state(AppState::Game)).after(facing_direction_system).after(flight_toggle_system).after(character_model_update_system));
    
    // Move speed command system
    app.add_systems(Update, move_speed_set_system.run_if(in_state(AppState::Game)));

    // Game systems - part 2
    app.add_systems(Update, (use_item_event_system.run_if(in_state(AppState::Game)),));
    app.add_systems(Update, (status_effect_system.run_if(in_state(AppState::Game)),));
    app.add_systems(Update, (passive_recovery_system.run_if(in_state(AppState::Game)),));
    app.add_systems(Update, (quest_trigger_system.run_if(in_state(AppState::Game)),));
    // game_mouse_input_system uses EguiContexts to check if egui wants pointer input
    app.add_systems(Update, game_mouse_input_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));
    // UI systems - part 1
    app.add_systems(Update, ui_bank_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_chatbox_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_character_info_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_clan_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_create_clan_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_inventory_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_game_menu_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_hotbar_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_minimap_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_npc_store_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_party_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_party_option_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_personal_store_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_player_info_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_quest_list_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));

    // UI systems - part 2
    app.add_systems(Update, ui_respawn_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_selected_target_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_skill_list_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_skill_tree_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_settings_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_status_effects_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, conversation_dialog_system.run_if(in_state(AppState::Game)).after(bevy_egui::EguiPreUpdateSet::InitContexts));

    if !systems_config.disable_player_command_system {
        app.add_systems(
            Update,
            player_command_system.run_if(in_state(AppState::Game)),
        );
    }

    // ui_drag_and_drop_system uses EguiContexts
    app.add_systems(Update, ui_drag_and_drop_system.after(bevy_egui::EguiPreUpdateSet::InitContexts));

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

    app.add_systems(PostStartup, load_common_game_data
        .after(bevy_egui::EguiStartupSet::InitContexts));
    
    // Create default particle texture before particle systems run
    app.add_systems(PostStartup, create_default_particle_texture);
    
    // TEST: Add StandardMaterial cube for rendering isolation test
    app.add_systems(PostStartup, spawn_test_cube);
    
    // DIAGNOSTIC: Print diagnostic summary on startup
    app.add_systems(PostStartup, print_diagnostic_summary.after(spawn_test_cube));

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
            app.add_systems(Startup, load_ui_resources
                .after(bevy_egui::EguiStartupSet::InitContexts));
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
        (GameStages::ZoneChange, GameStages::ZoneChangeFlush, GameStages::AfterUpdate)
            .before(PhysicsSet::SyncBackend),
    );

    app.configure_sets(
        PostUpdate,
        (GameStages::DebugRenderPreFlush, GameStages::DebugRender)
            .chain(),
    );

    // CRITICAL FIX: Use Bevy's default ordering for internal systems
    // Manual ordering of internal sets like VisibilityPropagate can break engine logic
    app.configure_sets(
        PostUpdate,
        GameStages::AfterUpdate.before(TransformSystem::TransformPropagate),
    );

    app.configure_sets(
        PostUpdate,
        VisibilitySystems::VisibilityPropagate.after(TransformSystem::TransformPropagate),
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
        (UiSystemSets::UiDebugMenu, UiSystemSets::UiFirst, UiSystemSets::Ui, UiSystemSets::UiLast, UiSystemSets::UiDebug)
            .in_set(GameSystemSets::Ui),
    );

    app.configure_sets(
        Update,
        (GameSystemSets::UpdateCamera, GameSystemSets::Ui),
    );

    // DIAGNOSTIC: Check if EguiContext exists on window entity
    app.add_systems(Update, |windows: Query<&EguiContext, With<Window>>| {
        if let Ok(_context) = windows.get_single() {
            //log::info!("[EGUI DIAGNOSTIC] EguiContext found on window entity");
        } else {
            log::warn!("[EGUI DIAGNOSTIC] EguiContext NOT found on window entity");
        }
    });

    // DIAGNOSTIC: Check if EguiRenderOutput exists on window entity
    app.add_systems(Update, |windows: Query<(&EguiContext, &EguiRenderOutput), With<Window>>| {
        if let Ok((_context, render_output)) = windows.get_single() {
            //log::info!("[EGUI DIAGNOSTIC] EguiRenderOutput found, paint_jobs count: {}", render_output.paint_jobs.len());
        } else {
            log::warn!("[EGUI DIAGNOSTIC] EguiRenderOutput NOT found on window entity");
        }
    });

    app.run();

    network_thread_tx.send(NetworkThreadMessage::Exit).ok();
    network_thread.join().ok();
}

fn load_game_data_irose(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
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
        animation_event_flags: rose_data_irose::get_animation_event_flags(),
        character_motion_database,
        client_strings: rose_data_irose::get_client_strings(string_database.clone())
            .expect("Failed to load client strings"),
        data_decoder: rose_data_irose::get_data_decoder(),
        effect_database: rose_data_irose::get_effect_database(&vfs_resource.vfs)
            .expect("Failed to load effect database"),
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
    mut egui_context: EguiContexts,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    //info!("[load_common_game_data] Starting to load common game data");

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
        )
        .expect("Failed to create model loader"),
    );

    //info!("[load_common_game_data] Spawning camera entity");
    let camera_entity = commands.spawn((
        Camera3d::default(),
        Msaa::Off,  // Required for SSAO and TAA compatibility
        Camera {
            hdr: true,  // Enable HDR for better depth of field
            clear_color: ClearColorConfig::Custom(Color::srgb(0.70, 0.90, 1.0)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::PI / 4.0,
            near: 0.1,
            far: 50000.0,
            aspect_ratio: 16.0 / 9.0,
        }),
        Transform::from_translation(Vec3::new(5120.0, 100.0, -5120.0))
            .looking_at(Vec3::new(5120.0, 0.0, -5130.0), Vec3::Y),
        GlobalTransform::default(),
        bevy::ui::IsDefaultUiCamera,
        // Add Tonemapping - REQUIRED for HDR to work properly with depth of field
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        // Add Bloom - enhances the depth of field effect visibility
        bevy::core_pipeline::bloom::Bloom::NATURAL,
        // Shadow filtering - Gaussian (non-temporal, works with SMAA)
        ShadowFilteringMethod::Gaussian,
        // SMAA for high-quality anti-aliasing without ghosting artifacts
        Smaa::default(),
        // Prepasses for depth (required for some effects and GPU occlusion culling)
        DepthPrepass,
        // GPU Occlusion Culling - Bevy 0.16 experimental feature
        // Culls objects hidden behind other objects to improve performance
        OcclusionCulling,
        // Underwater state tracking for underwater rendering effect
        CameraUnderwaterState::default(),
    )).id();
    // Insert additional components separately to avoid tuple size limit
    commands.entity(camera_entity).insert((
        // Bevy 0.16 built-in atmospheric scattering for realistic sky
        Atmosphere::EARTH,
        AtmosphereSettings::default(),
        // Add Depth of Field effect
        DepthOfField {
            mode: DepthOfFieldMode::Bokeh,
            focal_distance: 10.0,      // Focus 10 meters away
            aperture_f_stops: 3.3,     // f/3.3 aperture
            sensor_height: 0.01866,    // Super 35 format (default)
            max_circle_of_confusion_diameter: 64.0,
            max_depth: 2000.0,         // Max depth range
        },
        // Add VolumetricFog for light shafts/god rays effect
        // Configured for 60fps target with balanced quality
        VolumetricFog {
            ambient_intensity: 0.1,
            jitter: 0.0,
            step_count: 64,
            ..default()
        },
        // SSAO for contact shadows - adds darkening in crevices and where objects meet ground
        // Requires Msaa::Off (which is the default in Bevy 0.15)
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            constant_object_thickness: 0.25,  // Adjust if AO is too strong/weak
        },
        // Color Grading - filmic color correction for improved visual tone mapping
        // Provides exposure, contrast, saturation, and color tint controls
        ColorGrading {
            global: ColorGradingGlobal {
                exposure: 0.0,            // EV offset (0.0 = no change)
                temperature: 0.0,         // Warm/cooler (positive = warmer/redder)
                tint: 0.0,                // Green/magenta shift
                hue: 0.0,                 // Hue rotation in radians
                post_saturation: 1.1,     // Slightly increased saturation for vibrant colors
                midtones_range: 0.2..0.7, // Default midtone range
            },
            shadows: ColorGradingSection {
                saturation: 1.0,          // Keep shadow saturation
                contrast: 1.1,            // Slightly increased shadow contrast
                gamma: 1.0,               // No gamma adjustment
                gain: 1.0,                // No gain adjustment
                lift: 0.02,               // Slight lift to prevent crushed blacks
            },
            midtones: ColorGradingSection {
                saturation: 1.1,          // Increased midtone saturation
                contrast: 1.05,           // Slightly increased contrast
                gamma: 1.0,               // No gamma adjustment
                gain: 1.0,                // No gain adjustment
                lift: 0.0,                // No lift
            },
            highlights: ColorGradingSection {
                saturation: 0.95,         // Slightly reduced highlight saturation
                contrast: 1.0,            // Default contrast
                gamma: 1.0,               // No gamma adjustment
                gain: 1.05,               // Slight gain for brighter highlights
                lift: 0.0,                // No lift
            },
        },
    ));
    info!("[CAMERA] Camera entity spawned with id: {:?}", camera_entity);
    info!("[CAMERA] VolumetricFog settings: ambient_intensity=0.1, step_count=64");
    info!("[CAMERA] Shadow filtering: Gaussian (non-temporal)");
    info!("[CAMERA] SMAA enabled for anti-aliasing (no ghosting)");
    info!("[CAMERA] SSAO enabled with Medium quality for contact shadows");
    info!("[CAMERA] ColorGrading enabled with filmic color correction");
    info!("[CAMERA] GPU Occlusion Culling enabled (Bevy 0.16 experimental)");
    info!("[CAMERA] Camera position: ~5120.0, 100.0, -5120.0 (game world center)");

    commands.insert_resource(DamageDigitsSpawner::load(
        &asset_server,
        &mut meshes,
    ));

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Ubuntu-M".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!("fonts/Ubuntu-M.ttf"))),
    );

    fonts
        .families
        .entry(egui::FontFamily::Name("Ubuntu-M".into()))
        .or_default()
        .insert(0, "Ubuntu-M".to_owned());

    egui_context.ctx_mut().set_fonts(fonts);
}

/// Test cube spawn system for Bevy 0.14.2 rendering isolation test
/// This creates a simple red cube using StandardMaterial to verify core rendering works
fn spawn_test_cube(
    mut __commands__: Commands,
    mut __meshes__: ResMut<Assets<Mesh>>,
    mut __materials__: ResMut<Assets<StandardMaterial>>,
) {
    __commands__.spawn((
        Mesh3d(__meshes__.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(__materials__.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            ..default()
        })),
        Transform::from_xyz(5100.0, 75.0, -5100.0),
    ));
}

/// Camera extraction diagnostic system for Bevy 0.15.4
/// Logs camera state to verify extraction conditions are met
fn diagnose_camera_extraction_state(
    query: Query<(Entity, &Camera, &GlobalTransform, Option<&Camera3d>), With<Camera3d>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    // Log every 60 frames (~1 second at 60fps)
    if *frame_count % 60 != 0 {
        return;
    }
    
    let camera_count = query.iter().count();
    if camera_count == 0 {
        log::info!("[NAME_TAG_DEBUG] No Camera3d entities found for extraction diagnosis");
        return;
    }
    
    for (entity, camera, global_transform, camera3d) in query.iter() {
        let physical_viewport = camera.physical_viewport_rect();
        let physical_viewport_size = camera.physical_viewport_size();
        let physical_target_size = camera.physical_target_size();
        
        let has_valid_transform = global_transform.affine().translation.length() > 0.0
            || global_transform.affine().matrix3 != bevy::math::Mat3A::IDENTITY;
        
        log::info!(
            "[NAME_TAG_DEBUG] Camera entity {:?}: is_active={}, viewport={:?}, viewport_size={:?}, target_size={:?}, has_global_transform={}, has_camera3d={}",
            entity,
            camera.is_active,
            physical_viewport,
            physical_viewport_size,
            physical_target_size,
            has_valid_transform,
            camera3d.is_some()
        );
        
        // Log target info
        log::info!(
            "[NAME_TAG_DEBUG] Camera {:?} target: {:?}",
            entity,
            camera.target
        );
        
        // Check extraction conditions
        let viewport_ok = physical_viewport.is_some();
        let viewport_size_ok = physical_viewport_size.is_some();
        let target_size_ok = physical_target_size.is_some();
        let target_size_nonzero = physical_target_size.map(|s| s.x > 0 && s.y > 0).unwrap_or(false);
        
        log::info!(
            "[NAME_TAG_DEBUG] Camera {:?} extraction conditions: viewport_ok={}, viewport_size_ok={}, target_size_ok={}, target_size_nonzero={}",
            entity,
            viewport_ok,
            viewport_size_ok,
            target_size_ok,
            target_size_nonzero
        );
        
        if camera.is_active && viewport_ok && viewport_size_ok && target_size_ok && target_size_nonzero {
            log::info!("[NAME_TAG_DEBUG] Camera {:?} SHOULD be extracted successfully", entity);
        } else {
            log::warn!("[NAME_TAG_DEBUG] Camera {:?} may FAIL extraction - check conditions above", entity);
        }
    }
}

/// Diagnostic summary system for Bevy 0.14.2
/// Prints a comprehensive diagnostic summary on startup
fn print_diagnostic_summary(
    cameras: Query<&Camera>,
    meshes: Query<&Mesh3d>,
    render_diagnostics: Res<RenderExtractionDiagnostics>,
) {
    info!("=== BEVY 0.14.2 DIAGNOSTIC SUMMARY ===");
    info!("Active cameras: {}", cameras.iter().filter(|c| c.is_active).count());
    info!("Total mesh entities: {}", meshes.iter().count());
    info!("Main world meshes tracked: {}", render_diagnostics.main_world_mesh_count);
    info!("=======================================");
    info!("See docs/diagnostic-summary.md for interpretation guide");
}

/// System to apply depth of field settings from the resource to the camera
/// This allows live adjustment of DoF parameters via the Settings UI
fn apply_depth_of_field_settings(
    dof_settings: Res<DepthOfFieldSettings>,
    mut query: Query<&mut DepthOfField>,
) {
    use bevy::ecs::change_detection::DetectChanges;
    
    // Only update if settings have changed
    if dof_settings.is_changed() {
        for mut dof in query.iter_mut() {
            if dof_settings.enabled {
                dof.mode = dof_settings.mode;
                dof.focal_distance = dof_settings.focal_distance;
                dof.aperture_f_stops = dof_settings.aperture_f_stops;
                dof.sensor_height = dof_settings.sensor_height;
                dof.max_circle_of_confusion_diameter = dof_settings.max_circle_of_confusion_diameter;
                dof.max_depth = dof_settings.max_depth;
            } else {
                // When disabled, use Gaussian mode with minimal effect (effectively off)
                dof.mode = DepthOfFieldMode::Gaussian;
            }
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
    use bevy::pbr::DirectionalLight as DirectionalLightComponent;
    
    log::info!("[STARRY SKY] ========== SPAWN SYSTEM CALLED ==========");
    log::info!("[STARRY SKY] spawn_starry_sky_and_moon function executing");
    
    // CRITICAL: The sky sphere must be LARGE enough to contain the entire game world.
    // Camera is at ~5120, 100, -5120 which is ~7242 units from world origin.
    // Using 50000 units radius ensures camera is always inside the sphere.
    // The sphere is centered at world origin (0,0,0).
    let sky_sphere_radius = 50000.0;
    
    log::info!("[STARRY SKY] Sky sphere radius: {}", sky_sphere_radius);
    log::info!("[STARRY SKY] Camera expected at ~5120, 100, -5120 (inside sphere)");
    log::info!("[STARRY SKY] StarrySkySettings - star_density: {}, star_brightness: {}, night_factor: {}",
        starry_sky_settings.star_density,
        starry_sky_settings.star_brightness,
        starry_sky_settings.night_factor
    );
    
    // Create starry sky sphere mesh (large sphere centered at world origin)
    let sphere = Sphere::new(sky_sphere_radius);
    let mut sky_mesh = Mesh::from(sphere);
    log::info!("[STARRY SKY] Created sphere mesh primitive");
    
    // Flip normals for inside rendering (we're inside the sphere looking out)
    if let Some(normals) = sky_mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(normals) = normals {
            log::info!("[STARRY SKY] Flipping {} normals for inside rendering", normals.len());
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
            bevy::render::mesh::Indices::U32(indices) => {
                let count = indices.len() / 3;
                log::info!("[STARRY SKY] Reversing winding order for {} triangles", count);
                // Reverse each triangle (swap v1 and v2 of each triangle)
                for chunk in indices.chunks_mut(3) {
                    chunk.swap(1, 2);
                }
            }
            bevy::render::mesh::Indices::U16(indices) => {
                let count = indices.len() / 3;
                log::info!("[STARRY SKY] Reversing winding order for {} triangles (U16)", count);
                for chunk in indices.chunks_mut(3) {
                    chunk.swap(1, 2);
                }
            }
            _ => {
                log::warn!("[STARRY SKY] Unknown index format, cannot reverse winding order!");
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
    log::info!("[STARRY SKY] Created StarrySkyMaterial with time=0.0, night_factor={}", sky_material.night_factor);
    
    // Spawn starry sky entity
    let sky_mesh_handle = meshes.add(sky_mesh);
    let sky_material_handle = materials.add(sky_material);
    log::info!("[STARRY SKY] Mesh handle created: {:?}", sky_mesh_handle);
    log::info!("[STARRY SKY] Material handle created: {:?}", sky_material_handle);
    
    let sky_entity = commands.spawn((
        StarrySky,
        Mesh3d(sky_mesh_handle),
        MeshMaterial3d(sky_material_handle),
        Transform::from_xyz(0.0, 0.0, 0.0),  // Center of world - sphere is large enough to contain camera
        Visibility::Visible,
    )).id();
    
    log::info!("[STARRY SKY] StarrySky entity spawned with id: {:?}", sky_entity);
    log::info!("[STARRY SKY] Entity components: StarrySky, Mesh3d, MeshMaterial3d<StarrySkyMaterial>, Transform(0,0,0), Visibility::Visible");
    
    // Spawn moon directional light (separate from sun)
    // This provides illumination at night
    let moon_entity = commands.spawn((
        MoonLight,
        DirectionalLightComponent {
            illuminance: 5000.0,  // Moonlight intensity (much dimmer than sun)
            color: Color::srgb(0.8, 0.85, 0.95),  // Slightly blue-white moonlight
            shadows_enabled: true,
            shadow_depth_bias: 0.0,
            shadow_normal_bias: 0.0,
            affects_lightmapped_mesh_diffuse: true,
        },
        Transform::from_xyz(0.0, 100.0, 0.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        Visibility::Visible,
    )).id();
    
    log::info!("[STARRY SKY] MoonLight entity spawned with id: {:?}", moon_entity);
    log::info!("[STARRY SKY] ========== SPAWN COMPLETE ==========");
}

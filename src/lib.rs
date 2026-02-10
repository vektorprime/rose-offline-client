#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use animation::RoseAnimationPlugin;
use bevy::{
        asset::AssetApp,
        core_pipeline::bloom::BloomSettings,
        log::{info, warn, Level},
        pbr::{ExtendedMaterial, MaterialPlugin, StandardMaterial, MeshMaterial3d},
        prelude::{
            apply_deferred, default, in_state, resource_exists, App, AppExtStates, AssetServer, Assets, Camera, Camera3d,
            ClearColorConfig, Color, Commands, Cuboid, Entity, Handle, Image, InheritedVisibility, IntoSystemConfigs,
            IntoSystemSetConfigs, Local, Mesh, Mesh3d, Msaa, OnEnter, OnExit, PbrBundle, PerspectiveProjection, PluginGroup,
            PostStartup, PostUpdate, PreUpdate, Projection, Quat, Query, Res, ResMut, Startup, State,
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
use bevy_egui::{egui, EguiContext, EguiContexts, EguiRenderOutput};
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

use audio::OddioPlugin;
use diagnostics::RenderDiagnosticsPlugin;
use events::{
    BankEvent, CharacterSelectEvent, ChatboxEvent, ClanDialogEvent, ClientEntityEvent,
    ConversationDialogEvent, GameConnectionEvent, HitEvent, LoadZoneEvent, LoginEvent,
    MessageBoxEvent, MoveDestinationEffectEvent, NetworkEvent, NpcStoreEvent,
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
    SkyMaterialPlugin,
    TrailEffectRenderPlugin,
    WorldUiRenderPlugin,
    ZoneLightingPlugin,
    ExtensionMaterialPlugin,
};
use resources::{
    load_ui_resources, run_network_thread, ui_requested_cursor_apply_system, update_ui_resources,
    AppState, ClientEntityList, CurrentZone, DamageDigitsSpawner, DebugRenderConfig, GameData, NameTagSettings,
    NetworkThread, NetworkThreadMessage, RenderConfiguration, RenderExtractionDiagnostics, SelectedTarget, ServerConfiguration,
    SoundCache, SoundSettings, SpecularTexture, VfsResource, WorldTime, ZoneTime,
};
use scripting::RoseScriptingPlugin;
use systems::{
    ability_values_system, animation_effect_system, animation_sound_system, auto_login_system,
    background_music_system, character_model_add_collider_system, character_model_blink_system,
    character_model_update_system, character_select_enter_system, character_select_event_system,
    character_select_exit_system, character_select_input_system, character_select_models_system,
    character_select_system, clan_system, client_entity_event_system, collision_height_only_system,
    collision_player_system, collision_player_system_join_zoin, command_system,
    conversation_dialog_system, cooldown_system, damage_digit_render_system,
    debug_entity_visibility, debug_render_collider_system, debug_render_directional_light_system,
    debug_render_skeleton_system, directional_light_system, effect_system, facing_direction_system,
    // render_diagnostics_system_lightweight, frustum_culling_diagnostics,
    // material_transparency_diagnostics, transform_propagation_diagnostics,
    // transform_validation_diagnostics,
    // visibility_state_diagnostics, active_camera_diagnostics,
    // render_layer_diagnostics, aabb_validation_diagnostics,
    // render_pipeline_diagnostics, render_stage_diagnostics,
    // zone_entity_visibility_diagnostics, parent_child_visibility_diagnostics, zone_component_lifecycle_diagnostics,
    // diagnose_render_world_extraction, diagnose_render_phase, diagnose_camera_entity_distances,
    // verify_material_plugins,
    free_camera_system, game_connection_system, game_mouse_input_system, game_state_enter_system,
    game_zone_change_system, hit_event_system, item_drop_model_add_collider_system,
    item_drop_model_system, login_connection_system, login_event_system, login_state_enter_system,
    login_state_exit_system, login_system, model_viewer_enter_system, model_viewer_exit_system,
    model_viewer_system, move_destination_effect_system, name_tag_system,
    name_tag_update_color_system, name_tag_update_healthbar_system, name_tag_visibility_system,
    network_thread_system, npc_idle_sound_system, npc_model_add_collider_system,
    npc_model_update_system, orbit_camera_system, particle_sequence_system,
    passive_recovery_system, pending_damage_system, pending_skill_effect_system,
    personal_store_model_add_collider_system, personal_store_model_system, player_command_system,
    projectile_system, quest_trigger_system, spawn_effect_system, spawn_projectile_system,
    status_effect_system, system_func_event_system, update_position_system, use_item_event_system,
    vehicle_model_system, vehicle_sound_system, visible_status_effects_system,
    world_connection_system, world_time_system, zone_time_system, zone_viewer_enter_system,
    DebugInspectorPlugin,
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
    ui_window_sound_system, widgets::Dialog, DialogLoader, UiSoundEvent, UiStateDebugWindows,
    UiStateDragAndDrop, UiStateWindows,
};
use dds_image_loader::DdsImageLoader;
use vfs_asset_io::{VfsAssetIo, VfsAssetReaderPlugin};
use zms_asset_loader::{ZmsAssetLoader, ZmsMaterialNumFaces, ZmsNoSkinAssetLoader};
use zone_loader::{zone_loader_system, zone_loaded_from_vfs_system, force_zone_visibility_system, ZoneLoader, ZoneLoaderAsset, ZoneLoadChannelReceiver, ZoneLoadChannelSender, MemoryTrackingResource};

// Import diagnostic systems (ENABLED for debugging visibility issues)
// use systems::zone_memory_profiler_system::{
//     zone_memory_profiler_system, command_buffer_validation_system,
//     ZoneMemoryProfilerPlugin
// };
// DISABLED: use resources::zone_debug_diagnostics::{ZoneDebugDiagnostics, ZoneDebugDiagnosticsPlugin};

use crate::components::{CollisionPlayer, SoundCategory, Zone};

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
    pub fn create_virtual_filesystem(&self) -> Option<Arc<VirtualFilesystem>> {
        let mut vfs_devices: Vec<Box<dyn VirtualFilesystemDevice + Send + Sync>> = Vec::new();
        for device_config in self.devices.iter() {
            match device_config {
                FilesystemDeviceConfig::Directory(path) => {
                    log::info!("Loading game data from host directory {}", path);
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(path.into())));
                }
                FilesystemDeviceConfig::AruaVfs(path) => {
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);

                    log::info!("Loading game data from AruaVfs {}", path);
                    vfs_devices.push(Box::new(
                        AruaVfsIndex::load(Path::new(path), &index_root_path.join("data.rose"))
                            .unwrap_or_else(|_| panic!("Failed to load AruaVfs at {}", path)),
                    ));

                    log::info!(
                        "Loading game data from AruaVfs root path {}",
                        index_root_path.to_string_lossy()
                    );
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path)));
                }
                FilesystemDeviceConfig::TitanVfs(path) => {
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);

                    log::info!("Loading game data from TitanVfs {}", path);
                    vfs_devices.push(Box::new(
                        TitanVfsIndex::load(Path::new(path), &index_root_path.join("data.trf"))
                            .unwrap_or_else(|_| panic!("Failed to load TitanVfs at {}", path)),
                    ));

                    log::info!("Loading game data from TitanVfs root path {}", path);
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path)));
                }
                FilesystemDeviceConfig::Vfs(path) => {
                    log::info!("Loading game data from Vfs {}", path);
                    vfs_devices.push(Box::new(
                        VfsIndex::load(Path::new(path))
                            .unwrap_or_else(|_| panic!("Failed to load Vfs at {}", path)),
                    ));

                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);
                    log::info!("Loading game data from Vfs root path {}", path);
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path)));
                }
                FilesystemDeviceConfig::IrosePh(path) => {
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);

                    log::info!("Loading game data from iRosePH {}", path);
                    vfs_devices.push(Box::new(
                        IrosePhVfsIndex::load(Path::new(path))
                            .unwrap_or_else(|_| panic!("Failed to load iRosePH VFS at {}", path)),
                    ));

                    log::info!(
                        "Loading game data from iRosePH root path {}",
                        index_root_path.to_string_lossy()
                    );
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path)));
                }
            }
        }

        if vfs_devices.is_empty() {
            None
        } else {
            Some(Arc::new(VirtualFilesystem::new(vfs_devices)))
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
}

fn run_client(config: &Config, app_state: AppState, mut systems_config: SystemsConfig) {
    println!("run_client() function entered");
    log::info!("[VFS INIT] Starting VFS initialization...");
    log::info!("[VFS INIT] Config has {} filesystem devices", config.filesystem.devices.len());
    
    let virtual_filesystem =
        if let Some(virtual_filesystem) = config.filesystem.create_virtual_filesystem() {
            log::info!("[VFS INIT] VFS created successfully!");
            virtual_filesystem
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
                    level: bevy::log::Level::DEBUG,
                    filter: "wgpu=error,naga=error,bevy_render=debug,bevy_pbr=debug,bevy_asset=debug,rose_offline_client=trace,offset_allocator=warn".to_string(),
                    ..default()
                })
                .set(bevy::pbr::PbrPlugin::default()),
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
        ));

    // Initialise 3rd party bevy plugins
    // Note: RapierConfiguration is no longer a Resource in Bevy 0.15
    // Configuration is now handled through the RapierPhysicsPlugin
    app.add_plugins(bevy_egui::EguiPlugin);
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

    // DIAGNOSTIC: Initialize debug diagnostics resources (ENABLED for debugging visibility issues)
    // DISABLED: app.init_resource::<ZoneDebugDiagnostics>();
    app.init_resource::<RenderExtractionDiagnostics>();
    // app.init_resource::<crate::systems::zone_memory_profiler_system::ZoneMemoryProfiler>();
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
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>::default(),
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

    app.add_plugins((
            // Optional: Add these for full rendering support
            SkyMaterialPlugin { prepass_enabled: false },
            TrailEffectRenderPlugin,
            ZoneLightingPlugin,
            WorldUiRenderPlugin,

            RoseRenderPlugin,
            RoseScriptingPlugin,
            DebugInspectorPlugin,

            // Diagnostic plugins for debugging rendering crashes during zone loading
            RenderDiagnosticsPlugin,
        ));
    log::info!("[ASSET LOADER DIAGNOSTIC] Asset loaders registered successfully");

    // Material Plugin Diagnostic Logging
    log::info!("[MATERIAL PLUGIN] DamageDigitMaterialPlugin registered");
    log::info!("[MATERIAL PLUGIN] ParticleMaterialPlugin registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseObjectExtension> registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseTerrainExtension> registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseWaterExtension> registered");
    log::info!("[MATERIAL PLUGIN] ExtendedMaterial<StandardMaterial, RoseEffectExtension> registered");
    log::info!("[MATERIAL PLUGIN] SkyMaterialPlugin registered");

    // Setup state
    app.init_state::<AppState>();

    app.add_event::<BankEvent>()
        .add_event::<ChatboxEvent>()
        .add_event::<CharacterSelectEvent>()
        .add_event::<ClanDialogEvent>()
        .add_event::<ClientEntityEvent>()
        .add_event::<ConversationDialogEvent>()
        .add_event::<GameConnectionEvent>()
        .add_event::<HitEvent>()
        .add_event::<LoginEvent>()
        .add_event::<LoadZoneEvent>()
        .add_event::<MessageBoxEvent>()
        .add_event::<MoveDestinationEffectEvent>()
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

    app.add_systems(
        Update,
        (free_camera_system, orbit_camera_system).in_set(GameSystemSets::UpdateCamera),
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
            effect_system,
            animation_sound_system,
            npc_idle_sound_system,
            name_tag_system,
            character_model_update_system,
            character_model_add_collider_system,
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
            damage_digit_render_system,
            name_tag_update_healthbar_system,
            update_ui_resources,
            name_tag_visibility_system,
            name_tag_update_color_system,
            world_time_system,
            system_func_event_system,
            load_dialog_sprites_system,
            zone_time_system,
            directional_light_system,
        ),
    );

    app.add_systems(
        PostUpdate,
        ui_requested_cursor_apply_system,
    );

    app.add_systems(
        Update,
        ui_item_drop_name_system
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
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
    app.add_systems(Update, ui_debug_render_system
        .after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_skill_list_system
        .after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_zone_lighting_system
        .after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_zone_list_system
        .after(bevy_egui::EguiPreUpdateSet::InitContexts));
    app.add_systems(Update, ui_debug_zone_time_system
        .after(bevy_egui::EguiPreUpdateSet::InitContexts));
    // DISABLED: app.add_systems(Update, ui_debug_diagnostics_system);

    // character_model_blink_system in PostUpdate to avoid any conflicts with model destruction
    // e.g. through the character select exit system.
    app.add_systems(PostUpdate, character_model_blink_system);

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

    // FIX: Run force_zone_visibility_system BEFORE VisibilityPropagate
    // This ensures that Bevy's propagation system sees the updated Visibility
    // app.add_systems(
    //     PostUpdate,
    //     force_zone_visibility_system
    //         .before(VisibilitySystems::VisibilityPropagate),
    // );

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

    // DIAGNOSTIC: Add zone loading diagnostic systems (DISABLED - can be re-enabled for debugging)
    // These run after the zone loading systems to validate the results
    // app.add_systems(
    //     Update,
    //     (
    //         zone_memory_profiler_system,
    //         command_buffer_validation_system,
    //     )
    // );

    // Run debug render stage last after physics update so it has accurate data
    // DISABLED: debug_render_collider_system, debug_render_skeleton_system, debug_render_directional_light_system
    // app.add_systems(
    //     Update,
    //     (
    //         debug_render_collider_system,
    //         debug_render_skeleton_system,
    //         debug_render_directional_light_system,
    //     ),
    // );

    // Zone Viewer
    app.add_systems(OnEnter(AppState::ZoneViewer), zone_viewer_enter_system);
    // DISABLED: debug_entity_visibility
    // app.add_systems(
    //     Update,
    //     debug_entity_visibility
    //         .run_if(resource_exists::<CurrentZone>)
    //         .run_if(|time: Res<Time>| time.elapsed_seconds() % 5.0 < time.delta_seconds()),
    // );

    // Add render diagnostics system - runs every frame to check rendering state
    // DISABLED: app.add_systems(Update, render_diagnostics_system_lightweight);
    
    // // Add camera diagnostic system - verifies camera components for Bevy 0.14.2
    // app.add_systems(Update, diagnose_camera_system);
    
    // CRITICAL DIAGNOSTIC: Add Render World extraction diagnostic system
    // This system tracks how many entities are extracted from Main World to Render World
    // This is CRITICAL because Main World visibility does NOT guarantee Render World extraction
    // DISABLED
    // if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
    //     bevy::log::info!("[RENDER WORLD DIAGNOSTIC] Initializing render world extraction diagnostics");
    //     render_app.init_resource::<RenderExtractionDiagnostics>();
    //     render_app.add_systems(ExtractSchedule, diagnose_render_world_extraction);
    // } else {
    //     bevy::log::error!("[RENDER WORLD DIAGNOSTIC] FAILED to get render app - extraction diagnostics will not run!");
    // }
    
    // CRITICAL DIAGNOSTIC: Add Render Phase diagnostic system
    // This system checks if render queues (Opaque3d, Transparent3d) have items
    // Empty render queues indicate extraction failure or culling issues
    // DISABLED
    // if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
    //     bevy::log::info!("[RENDER PHASE DIAGNOSTIC] Initializing render phase diagnostics");
    //     render_app.add_systems(Render, diagnose_render_phase);
    // } else {
    //     bevy::log::error!("[RENDER PHASE DIAGNOSTIC] FAILED to get render app - phase diagnostics will not run!");
    // }
    
    // CRITICAL DIAGNOSTIC: Add camera-entity distance diagnostic system
    // This system verifies that visible entities are within reasonable distance of camera
    // Helps identify entities that are "visible" but outside camera frustum
    // DISABLED: app.add_systems(Update, diagnose_camera_entity_distances);
    
    // CRITICAL DIAGNOSTIC: Add material plugin verification system
    // This system verifies that Bevy's built-in MaterialPlugin is properly extracting materials
    // Helps diagnose if custom materials are interfering with StandardMaterial extraction
    // DISABLED: app.add_systems(Update, verify_material_plugins);

    // CRITICAL DIAGNOSTIC: Add Main World mesh diagnostic system
    // This verifies that meshes exist in Main World with proper visibility
    // Runs periodically to avoid log spam (every 60 frames ~ 1 second at 60fps)
    // DISABLED
    // app.add_systems(
    //     Update,
    //     diagnose_main_world_meshes
    //         .run_if(|time: Res<Time>| time.elapsed_seconds() % 2.0 < time.delta_seconds()),
    // );

    // CRITICAL DIAGNOSTIC: Add mesh material diagnostic system
    // This verifies that meshes have materials assigned, which is required for rendering
    // Note: RenderPhase diagnostics require render-world access and are handled separately
    // Runs periodically (every 2 seconds) to avoid log spam
    // DISABLED
    // app.add_systems(
    //     Update,
    //     diagnose_mesh_materials
    //         .run_if(|time: Res<Time>| time.elapsed_seconds() % 2.0 < time.delta_seconds()),
    // );

    // Add comprehensive diagnostic systems for debugging rendering issues
    // DISABLED
    // app.add_systems(Update, (
    //     frustum_culling_diagnostics,
    //     material_transparency_diagnostics,
    //     transform_validation_diagnostics,
    //     visibility_state_diagnostics,
    //     active_camera_diagnostics,
    //     render_layer_diagnostics,
    //     aabb_validation_diagnostics,
    //     render_pipeline_diagnostics,
    //     render_stage_diagnostics,
    // ));

    // GPU BUFFER UPLOAD DIAGNOSTICS - Phase 6
    // These systems verify that mesh and material data is actually being uploaded to the GPU
    // Runs every 3 seconds to avoid log spam
    // DISABLED
    // app.add_systems(
    //     Update,
    //     (
    //         diagnose_gpu_mesh_upload,
    //         diagnose_asset_loading,
    //     ).run_if(|time: Res<Time>| time.elapsed_seconds() % 3.0 < time.delta_seconds()),
    // );

    // CRITICAL DIAGNOSTIC: Add transform propagation diagnostics
    // This will tell us if transform propagation is actually running
    // DISABLED: app.add_systems(Update, transform_propagation_diagnostics);

    // CRITICAL DIAGNOSTIC: Add zone entity visibility diagnostics
    // This will help diagnose why entities are not visible in the zone
    // DISABLED per user request
    // app.add_systems(
    //     PostUpdate,
    //     (
    //         zone_entity_visibility_diagnostics,
    //         |query: Query<(Entity, &ViewVisibility, &Visibility), With<Zone>>| {
    //             for (entity, view_vis, vis) in query.iter() {
    //                 // info!("[ZONE VISIBILITY CHECK] Entity: {:?}, Visibility: {:?}, ViewVisibility: {}",
    //                 //     entity, vis, view_vis.get());
    //             }
    //         }
    //     ).chain().in_set(GameStages::DebugRender)
    // );

    // CRITICAL DIAGNOSTIC: Add parent-child visibility diagnostics
    // This will help diagnose hierarchy visibility issues
    // DISABLED: app.add_systems(PostUpdate, parent_child_visibility_diagnostics.in_set(GameStages::DebugRender));

    // CRITICAL DIAGNOSTIC: Add zone component lifecycle diagnostics
    // This will help diagnose why Zone component is missing
    // DISABLED: app.add_systems(PostUpdate, zone_component_lifecycle_diagnostics.in_set(GameStages::DebugRender));

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
        .run_if(in_state(AppState::GameLogin)),
    );

    app.add_systems(
        Update,
        (ui_login_system, ui_server_select_system).run_if(in_state(AppState::GameLogin)).in_set(UiSystemSets::Ui)
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

    app.add_systems(
        Update,
        (
            character_select_system,
            character_select_input_system,
            character_select_models_system,
            character_select_event_system,
        )
            .run_if(in_state(AppState::GameCharacterSelect)),
    );

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

    // Game
    app.init_resource::<UiStateDragAndDrop>()
        .init_resource::<UiStateWindows>()
        .init_resource::<UiStateDebugWindows>()
        .init_resource::<ClientEntityList>()
        .init_resource::<DebugRenderConfig>()
        .init_resource::<WorldTime>()
        .init_resource::<ZoneTime>()
        .init_resource::<SelectedTarget>()
        .init_resource::<NameTagSettings>();

    app.add_systems(OnEnter(AppState::Game), game_state_enter_system);

    // Register systems individually to avoid Bevy 0.13's IntoSystemConfigs trait bound issues
    // Game systems - part 1
    app.add_systems(Update, ability_values_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, clan_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, command_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, facing_direction_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, update_position_system.run_if(in_state(AppState::Game)));
    // app.add_systems(Update, collision_player_system_join_zoin.run_if(in_state(AppState::Game))
    //     .before(collision_player_system));
    app.add_systems(Update, collision_height_only_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, collision_player_system.run_if(in_state(AppState::Game)));
    app.add_systems(
        Update,
        collision_player_system_join_zoin
            .run_if(in_state(AppState::Game))
            .after(collision_player_system),
    );
    app.add_systems(Update, cooldown_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, client_entity_event_system.run_if(in_state(AppState::Game)));

    // Game systems - part 2
    app.add_systems(Update, (use_item_event_system.run_if(in_state(AppState::Game)),));
    app.add_systems(Update, (status_effect_system.run_if(in_state(AppState::Game)),));
    app.add_systems(Update, (passive_recovery_system.run_if(in_state(AppState::Game)),));
    app.add_systems(Update, (quest_trigger_system.run_if(in_state(AppState::Game)),));
    // app.add_systems(Update, game_mouse_input_system); // Too many parameters for Bevy 0.15
    // need to review if the game_mouse_input_system was added another way
    // UI systems - part 1
    app.add_systems(Update, ui_bank_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_chatbox_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_character_info_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_clan_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_create_clan_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_inventory_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_game_menu_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_hotbar_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_minimap_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_npc_store_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_party_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_party_option_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_personal_store_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_player_info_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_quest_list_system.run_if(in_state(AppState::Game)));

    // UI systems - part 2
    app.add_systems(Update, ui_respawn_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_selected_target_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_skill_list_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_skill_tree_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_settings_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, ui_status_effects_system.run_if(in_state(AppState::Game)));
    app.add_systems(Update, conversation_dialog_system.run_if(in_state(AppState::Game))
        .after(bevy_egui::EguiPreUpdateSet::InitContexts));

    if !systems_config.disable_player_command_system {
        app.add_systems(
            Update,
            player_command_system.run_if(in_state(AppState::Game)),
        );
    }

    app.add_systems(PostUpdate, ui_drag_and_drop_system);

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
            game_connection_system,
        ),
    );

    app.add_systems(PostStartup, load_common_game_data
        .after(bevy_egui::EguiStartupSet::InitContexts));
    
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
    // app.configure_sets(
    //     PostUpdate,
    //     GameStages::DebugRenderPreFlush.after(VisibilitySystems::CheckVisibility),
    // );

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
            .in_set(GameSystemSets::Ui)
            .after(bevy_egui::EguiPreUpdateSet::InitContexts),
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
    mut damage_digit_materials: ResMut<Assets<DamageDigitMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    bevy::log::info!("[load_common_game_data] Starting to load common game data");

    commands.insert_resource(SpecularTexture {
        image: asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"),
    });

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

    bevy::log::info!("[load_common_game_data] Spawning camera entity");
    // let camera_entity = commands.spawn((
    //     Camera3d::default(),
    //     Camera {
    //         hdr: false,
    //         clear_color: ClearColorConfig::Custom(Color::srgb(0.70, 0.90, 1.0)),
    //         ..Default::default()
    //     },
    //     Projection::from(PerspectiveProjection {
    //         fov: std::f32::consts::PI / 4.0,
    //         near: 0.1,
    //         far: 50000.0,
    //         aspect_ratio: 16.0 / 9.0,
    //     }),
    //     // Camera positioned for optimal zone viewing
    //     // Zone center is approximately (5200.0, 0.0, -5200.0)
    //     // Position camera at a 45-degree angle, 200 units away, at a reasonable height
    //     Transform::from_translation(Vec3::new(5120.0, 100.0, -5120.0))
    //         .looking_at(Vec3::new(5120.0, 0.0, -5130.0), Vec3::Y),
    //     GlobalTransform::default(),
    //     Visibility::default(),
    //     InheritedVisibility::default(),
    //     ViewVisibility::default(),
    //     bevy::render::view::RenderLayers::layer(0),
    // )).id();
    let camera_entity = commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: false,
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
        bevy::ui::IsDefaultUiCamera,
    )).id();
    bevy::log::info!("[load_common_game_data] Camera entity spawned with id: {:?}", camera_entity);

    commands.insert_resource(DamageDigitsSpawner::load(
        &asset_server,
        &mut damage_digit_materials,
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

/// Camera diagnostic system for Bevy 0.14.2
/// Verifies all required camera components are present and configured correctly
fn diagnose_camera_system(
    cameras: Query<(
        Entity,
        &Camera,
        Option<&Camera3d>,
        Option<&Projection>,
        &Transform,
        Option<&Visibility>,
        Option<&InheritedVisibility>,
        Option<&ViewVisibility>,
    )>,
) {
    let camera_count = cameras.iter().count();
    info!("[CAMERA DIAGNOSTIC] Active cameras found: {}", camera_count);
    
    for (entity, camera, cam3d, projection, transform, vis, inherited, view) in cameras.iter() {
        info!("[CAMERA DIAGNOSTIC] Camera {:?}:", entity);
        info!("  - is_active: {}", camera.is_active);
        info!("  - has Camera3d marker: {}", cam3d.is_some());
        info!("  - has Projection: {}", projection.is_some());
        info!("  - position: {:?}", transform.translation);
        info!("  - has Visibility: {}", vis.is_some());
        info!("  - has InheritedVisibility: {}", inherited.is_some());
        info!("  - has ViewVisibility: {}", view.is_some());
        
        if let Some(proj) = projection {
            match proj {
                Projection::Perspective(p) => {
                    info!("  - fov: {}", p.fov);
                }
                Projection::Orthographic(_) => {
                    info!("  - orthographic projection");
                }
            }
        }
    }
}

/// DISABLED: Main World mesh diagnostic system for Bevy 0.14.2
/// Verifies that meshes exist in Main World with proper visibility settings
/// This is critical because Main World visibility does NOT guarantee Render World extraction
#[allow(dead_code)]
fn diagnose_main_world_meshes(
    meshes: Query<(Entity, &Mesh3d, &Transform, &Visibility)>,
    mut diagnostics: ResMut<RenderExtractionDiagnostics>,
) {
    let mesh_count = meshes.iter().count();
    
    // Update diagnostics
    diagnostics.main_world_mesh_count = mesh_count;
    
    // Count visibility states
    let mut visible_count = 0;
    let mut inherited_count = 0;
    let mut hidden_count = 0;
    
    for (_, _, _, visibility) in meshes.iter() {
        match visibility {
            Visibility::Visible => visible_count += 1,
            Visibility::Inherited => inherited_count += 1,
            Visibility::Hidden => hidden_count += 1,
        }
    }
    
    diagnostics.meshes_marked_visible = visible_count;
    diagnostics.meshes_with_inherited_visibility = inherited_count;
    diagnostics.meshes_with_hidden_visibility = hidden_count;
    
    info!("[MAIN WORLD] Mesh diagnostic summary:");
    info!("[MAIN WORLD] Total meshes with Visibility: {}", mesh_count);
    info!("[MAIN WORLD] Meshes with Visibility::Visible: {}", visible_count);
    info!("[MAIN WORLD] Meshes with Visibility::Inherited: {}", inherited_count);
    info!("[MAIN WORLD] Meshes with Visibility::Hidden: {}", hidden_count);
    
    // Show details for first 5 meshes
    for (entity, _mesh_handle, transform, visibility) in meshes.iter().take(5) {
        let is_visible = *visibility == Visibility::Visible;
        info!(
            "[MAIN WORLD] Mesh {:?}: visible={:?} (is_visible={}), pos={:?}",
            entity,
            visibility,
            is_visible,
            transform.translation
        );
    }
    
    if mesh_count > 5 {
        info!("[MAIN WORLD] ... and {} more meshes", mesh_count - 5);
    }
    
    // Critical check: Alert if meshes exist but none are explicitly Visible
    if mesh_count > 0 && visible_count == 0 {
        log::warn!(
            "[MAIN WORLD] CRITICAL: {} meshes exist but NONE have Visibility::Visible! \
            Render extraction may fail if parent entities aren't visible.",
            mesh_count
        );
    }
}

/// DISABLED: Mesh material diagnostic system for Bevy 0.14.2
/// Checks if meshes have materials assigned, which is required for rendering
#[allow(dead_code)]
fn diagnose_mesh_materials(
    meshes_with_materials: Query<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
    meshes_with_custom_materials: Query<(Entity, &Mesh3d), Without<MeshMaterial3d<StandardMaterial>>>,
) {
    let standard_count = meshes_with_materials.iter().count();
    let custom_count = meshes_with_custom_materials.iter().count();
    
    info!("[MATERIAL DIAGNOSTIC] Meshes with StandardMaterial: {}", standard_count);
    info!("[MATERIAL DIAGNOSTIC] Meshes without StandardMaterial: {}", custom_count);
    
    if standard_count == 0 && custom_count == 0 {
        warn!("[MATERIAL DIAGNOSTIC] WARNING: No meshes with materials found!");
    }
}

/// DISABLED: GPU mesh upload diagnostic system for Bevy 0.14.2
/// Verifies that mesh vertex buffers have been uploaded to the GPU
/// This is critical because mesh data must be in GPU memory to render
///
/// Note: We check the number of meshes in the Assets<Mesh> resource and compare with
/// the number of entities with mesh handles. The actual GPU upload status is tracked
/// by Bevy's render asset system.
#[allow(dead_code)]
fn diagnose_gpu_mesh_upload(
    meshes_assets: Res<Assets<Mesh>>,
    meshes: Query<(Entity, &Mesh3d, &Transform)>,
) {
    let total_meshes = meshes.iter().count();
    let loaded_mesh_count = meshes_assets.iter().count();
    let mut not_ready: Vec<Entity> = Vec::new();
    
    // Check if mesh handles reference loaded meshes
    let mut mesh_ready_count = 0;
    for (entity, handle, _transform) in meshes.iter().take(10) {
        if meshes_assets.get(handle).is_some() {
            mesh_ready_count += 1;
        } else {
            not_ready.push(entity);
        }
    }
    
    info!("[GPU MESH] Total mesh entities: {}, Loaded mesh assets: {}, Ready: {}/10 sampled", 
        total_meshes, loaded_mesh_count, mesh_ready_count);
    
    if !not_ready.is_empty() {
        warn!("[GPU MESH] Meshes not yet loaded: {:?}", not_ready);
    }
    
    if mesh_ready_count == 0 && total_meshes > 0 {
        warn!("[GPU MESH] CRITICAL: No meshes are loaded but {} mesh entities exist in world!", total_meshes);
    }
}

/// DISABLED: Asset loading diagnostic system for Bevy 0.14.2
/// Checks if meshes and materials are loaded and available
#[allow(dead_code)]
fn diagnose_asset_loading(
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
) {
    info!("[ASSET] Meshes loaded: {}", meshes.iter().count());
    info!("[ASSET] StandardMaterials loaded: {}", materials.iter().count());
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

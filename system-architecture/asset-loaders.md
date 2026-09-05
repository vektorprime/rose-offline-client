# Asset Loaders (VFS, Models, Textures, Dialogs, Effects)

Zone files bypass the `AssetServer` (see [zone-pipeline.md](zone-pipeline.md)). Everything else loads through the custom `AssetReader` + format loaders below.

## VFS gateway (`src/vfs_asset_io.rs`)

- `VfsAssetIo` is the `AssetSourceId::Default` reader (`VfsAssetReaderPlugin`). No other asset source is registered.
- Priority: global `VFS_FILE_CACHE` → `base_path` real files (local overrides win) → VFS archive (`AruaVfs` / `TitanVfs` / `IrosePh` / `FilesystemDeviceConfig::Vfs`) → local fallback. Paths with `shaders/` bypass VFS so `load_internal_asset!` keeps working.
- `read_directory` returns an empty stream to keep Bevy hot-reload from scanning the VFS into a reload loop; `clear_vfs_file_cache()` runs on zone transitions.
- Bootstrapping (`src/main.rs`): `--data-idx`, `--data-aruavfs-idx`, `--data-titanvfs-idx`, `--data-iroseph-idx`, `--data-path`, else `./data.idx`.

## Format loaders

- ZMS meshes (`src/zms_asset_loader.rs`): position/normal/tangent/color/joint weights+indices, UV1–UV4; `ZmsNoSkinAssetLoader` (`.no_skin`) for effect meshes to avoid skinned bind-group mismatches.
- ZMO motion (`src/animation/zmo_asset_loader.rs`): `.zmo` skeletal/transform/camera + `.zmo_texture` morph textures; see [Animation.md](Animation.md).
- DDS (`src/dds_image_loader.rs`): BC1–BC3/DXT1–DXT5 plus uncompressed/alpha-luminance fallbacks, all uploaded RGBA8; `#cube` label loads cubemaps (used by `EnvironmentMapLight`).
- EXE resources (`src/exe_resource_loader.rs`): `trose.exe#cursor_*` cursors and sprites backing `UiResources` (`src/resources/ui_resources.rs`). Cursor assets load but no `CursorIcon::Custom` is inserted yet — see [Window.md](Window.md).
- Models/materials (`src/model_loader.rs`): ZSC-driven assembly of the above into `CharacterModel`/`NpcModel` parts.
- Dialogs (`src/ui/dialog_loader.rs`): `quick_xml` → `Dialog{Vec<Widget>}`; widgets in `src/ui/widgets/`; see [UI.md](UI.md).
- Effects (`src/effect_loader.rs`): `EffectCache` for parsed `EftFile`s; GPU rendering via `ParticleMaterial`; combat wiring in [combat-effects.md](combat-effects.md).

## Logging

- Sessions write `logs/<timestamp>/{structured.jsonl,session.json}` (`src/logging/`); the user runs the client, not the agent.

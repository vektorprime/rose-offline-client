# Asset Loaders (VFS, Models, Textures, Dialogs, Effects)

Zone files bypass the `AssetServer` (see [zone-pipeline.md](zone-pipeline.md)). Everything else loads through the custom `AssetReader` + format loaders below.

## VFS gateway (`src/vfs_asset_io.rs`)

- `VfsAssetIo` is the `AssetSourceId::Default` reader (`VfsAssetReaderPlugin`). No other asset source is registered.
- Priority: `base_path` real files (local overrides win) → VFS archive (`AruaVfs` / `TitanVfs` / `IrosePh` / `FilesystemDeviceConfig::Vfs`) → local fallback. Paths with `shaders/` bypass VFS so `load_internal_asset!` keeps working.
- No copies: memory-mapped archive entries (the default `data.idx`) are handed to loaders as a `SliceReader` over the mmap; owned buffers (decrypted/decompressed entries, real files) move into a `VecReader`. There is no file cache (the old global cache kept every asset's raw bytes until the next zone change; Bevy already dedupes loads by handle).
- `read_directory` returns an empty stream to keep Bevy hot-reload from scanning the VFS into a reload loop.
- Bootstrapping (`src/main.rs`): `--data-idx`, `--data-aruavfs-idx`, `--data-titanvfs-idx`, `--data-iroseph-idx`, `--data-path`, else `./data.idx`.

## Format loaders

- ZMS meshes (`src/zms_asset_loader.rs`): position/normal/tangent/color/joint weights+indices, UV1–UV4; `ZmsNoSkinAssetLoader` (`.no_skin`) for effect meshes to avoid skinned bind-group mismatches.
- ZMO motion (`src/animation/zmo_asset_loader.rs`): `.zmo` skeletal/transform/camera + `.zmo_texture` morph textures; see [Animation.md](Animation.md).
- DDS (`src/dds_image_loader.rs`, registered by `DdsImageLoaderPlugin` in `finish()` once GPU BC support is known): BC1–BC3/DXT1–DXT5 are uploaded compressed (`Bc{1,2,3}RgbaUnormSrgb`) when the GPU supports BC, so the hardware decodes them. The full mip chain from the file is uploaded on both paths (each level checked against the file length; a truncated chain keeps its complete levels), except UI textures and `#cube` cubemaps, which stay single-level. Samplers are trilinear like the original client; the address mode follows the original (`zz_material.cpp`): base textures repeat, lightmap pages (`/LIGHTMAP/`), UI textures and cubemaps clamp. Formats: BC1-3, 24/32-bit RGB(A)/BGR(A) (X8 variants opaque), 565 in both channel orders, 1555, 4444, A8, L8, L8A8 and DXGI `R32G32B32A32_FLOAT`; BC4-BC7 are rejected with an error instead of being misread. CPU decode to RGBA8 remains for: no BC support, sizes not a multiple of 4, the `#cube` label (cubemaps, used by `EnvironmentMapLight`), and UI textures under `3ddata/control/` (the UI premultiplies their alpha on the CPU). Other formats (uncompressed, alpha/luminance) are always converted to RGBA8. Note the CPU BC2/BC3 decoder used BC1's 3-colour mode when `color0 <= color1`; hardware (and the original client) always decodes BC2/BC3 colour blocks in 4-colour mode.
- EXE resources (`src/exe_resource_loader.rs`): `trose.exe#cursor_*` cursors and sprites backing `UiResources` (`src/resources/ui_resources.rs`). Cursor assets load but no `CursorIcon::Custom` is inserted yet — see [Window.md](Window.md).
- Models/materials (`src/model_loader.rs`): ZSC-driven assembly of the above into `CharacterModel`/`NpcModel` parts.
- Dialogs (`src/ui/dialog_loader.rs`): `quick_xml` → `Dialog{Vec<Widget>}`; widgets in `src/ui/widgets/`; see [UI.md](UI.md).
- Effects (`src/effect_loader.rs`): `EffectCache` for parsed `EftFile`s and parsed `PtlFile`s (particle files used to be re-read per particle per spawn; `ParticleSequence::from_ref` builds a sequence from the shared file); GPU rendering via `ParticleMaterial`; combat wiring in [combat-effects.md](combat-effects.md).

## Logging

- Sessions write `logs/<timestamp>/{structured.jsonl,session.json}` (`src/logging/`); the user runs the client, not the agent.

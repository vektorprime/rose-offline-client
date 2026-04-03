# Audio System Architecture

## Overview
The project utilizes a custom audio implementation based on `oddio` and `cpal` instead of Bevy's built-in audio system. This custom architecture provides granular control over audio mixing, spatialization, and streaming, allowing for advanced features like precise gain control, custom crossfading for background music, and efficient handling of large sound libraries via a custom cache.

## Core Components

### OddioPlugin
The `OddioPlugin` handles the initialization of the audio backend. It sets up the `cpal` output stream and integrates the `oddio` mixer and spatial scene into Bevy.

- **Initialization**: It detects the default output device and sample rate, then creates a root mixer and a spatial scene.
- **Integration**: The `OddioContext` resource is inserted to provide access to the mixer and spatial scene handles across the application.

Example: `src/audio/mod.rs:60`
```rust
impl Plugin for OddioPlugin {
    fn build(&self, app: &mut App) {
        // ... cpal device setup ...
        let (mut root_mixer_handle, root_mixer) = oddio::split(oddio::Mixer::new());
        let (scene_handle, scene) = oddio::split(oddio::SpatialScene::new());
        root_mixer_handle.control().play(scene);
        // ... stream setup ...
        app.insert_non_send_resource(stream)
            .insert_resource(OddioContext {
                mixer: root_mixer_handle,
                spatial: scene_handle,
                sample_rate: sample_rate.0,
            })
            // ...
    }
}
```

### SoundCache
`SoundCache` implements an asset caching strategy to prevent redundant loading of sound files. It uses a `RwLock` protected vector of handles indexed by `SoundId`.

- **Strategy**: When a sound is requested, the cache is checked first. If not found, the `AssetServer` loads the file, and the resulting handle is stored in the cache.

Example: `src/resources/sound_cache.rs:28`
```rust
pub fn load(&self, sound_data: &SoundData, asset_server: &AssetServer) -> Handle<AudioSource> {
    if let Some(cached) = self.get(sound_data.id) {
        return cached;
    }

    let handle = asset_server.load(sound_data.path.path().to_string_lossy().into_owned());
    self.set(sound_data.id, handle.clone());
    handle
}
```

### SoundSettings
`SoundSettings` manages volume configuration through categories. It uses an `EnumMap` to store gain values for different categories of sound.

- **Gain Calculation**: The final gain is calculated as the product of the `global_gain` and the category-specific gain.

Example: `src/resources/sound_settings.rs:14`
```rust
impl SoundSettings {
    pub fn gain(&self, category: SoundCategory) -> SoundGain {
        if self.enabled {
            SoundGain::Ratio(self.global_gain * self.gains[category])
        } else {
            SoundGain::Ratio(0.0)
        }
    }
}
```

## Sound Categories
Audio is divided into the following categories for independent volume control:
- `BackgroundMusic`: Zone-specific ambient music.
- `PlayerFootstep`: Sounds produced by the player's movement.
- `PlayerCombat`: Sounds produced by player attacks and abilities.
- `OtherFootstep`: Sounds produced by NPCs or other players' movement.
- `OtherCombat`: Sounds produced by NPCs or other players' combat.
- `NpcSounds`: Ambient and dialogue sounds from NPCs.
- `Ui`: Interface interaction sounds.

## Spatial vs Global Audio

### Spatial Audio (`SpatialSound`)
Used for entities in the 3D world. It calculates audio positioning relative to the listener (usually the player).
- **Positional Logic**: The system adjusts the sound position to be in the direction of the camera but maintains the distance from the player character.
- **Velocity**: Doppler effects are approximated by calculating the relative velocity between the sound source and the listener.

Example: `src/audio/spatial_sound.rs:156`
```rust
let spatial_position = (sound_global_translation - camera_position).normalize()
    * (sound_global_translation - listener_position).length();
```

### Global Audio (`GlobalSound`)
Used for UI elements and Background Music. Global sounds are played directly through the root mixer without 3D positioning.

## Key Systems

### `background_music_system`
Manages zone-based BGM with day/night cycles and crossfading.
- **Logic**: Monitors `CurrentZone` and `ZoneTime`. When the zone or time of day changes, it triggers a crossfade by fading out the current track and fading in the new one over a set duration.

Example: `src/systems/background_music_system.rs:10`
```rust
const CROSSFADE_DURATION_MS: u64 = 2000;
```

### `animation_sound_system`
Plays sound effects tied to specific animation frames.

### `npc_idle_sound_system`
Handles ambient sounds emitted by NPCs when they are in an idle state.

### `vehicle_sound_system`
Manages audio for vehicles, typically including engine loops and movement sounds.

## Troubleshooting

### Oddio Initialization Failures
- **No Output Device**: Ensure a valid audio output device is connected. The `OddioPlugin` will panic if `default_output_device()` returns `None`.
- **Sample Rate Mismatch**: Verify that the device supports the sample rate requested by the configuration.

### Spatial Audio Positioning Issues
- **Listener Position**: Ensure the `PlayerCharacter` component is present; otherwise, the system falls back to the `Camera3d` position, which may cause audio to feel disconnected from the player.
- **Radius**: Check the `SoundRadius` component; if not provided, it defaults to 4.0 units.

### Memory Usage
- **Cache Bloat**: The `SoundCache` uses a fixed-size vector. Ensure the size is appropriate for the number of unique sounds in the game to avoid out-of-bounds access or excessive memory allocation.
- **Unused Handles**: Ensure that `GlobalSound` and `SpatialSound` entities are despawned when they finish playing to release asset handles.

## Source File References
- **Core Logic**: `src/audio/mod.rs`
- **Resources**: `src/resources/sound_cache.rs`, `src/resources/sound_settings.rs`
- **Systems**: `src/systems/background_music_system.rs`, `src/systems/animation_sound_system.rs`, `src/systems/npc_idle_sound_system.rs`, `src/systems/vehicle_sound_system.rs`
- **Audio Types**: `src/audio/global_sound.rs`, `src/audio/spatial_sound.rs`, `src/audio/audio_source.rs`
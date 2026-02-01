# WGPU Adapter and Surface Diagnostics

> **Version**: 1.0 | **Bevy Version**: 0.13.2 | **WGPU Version**: 0.19  
> **Purpose**: Deep diagnostics for WGPU graphics initialization, adapter selection, surface configuration, and presentation

---

## 1. Executive Summary

This document provides exhaustive diagnostics for the WGPU graphics layer used by Bevy 0.13.2. Black screens can originate from failures at the GPU initialization layer before any rendering occurs. This guide covers:

- **Adapter Selection**: GPU enumeration, backend selection, feature detection
- **Device/Queue Setup**: Device creation, limits validation, lost device handling
- **Surface Configuration**: Swapchain format, present modes, alpha handling
- **Presentation Monitoring**: Surface texture acquisition, timeout detection, recreation
- **Backend-Specific Issues**: Vulkan validation, DirectX 12 debugging, Metal diagnostics

### Relationship to Main Diagnostic Protocol

This document is a **Phase P2 (Window & Graphics Context)** deep-dive supplement to the [`black-screen-diagnostic-protocol.md`](black-screen-diagnostic-protocol.md). Use this when:
- Standard diagnostics show window creation succeeds but rendering fails
- Console shows WGPU-related errors or panics
- Need to diagnose GPU driver issues, backend selection problems
- Surface-related errors appear (outdated, lost, timeout)

---

## 2. WGPU Adapter Selection Diagnostics

### 2.1 Environment Variable Configuration

WGPU and Bevy 0.13.2 respect these environment variables for backend and adapter control:

```powershell
# Windows PowerShell/Command Prompt
# Force specific backend (useful for testing fallback paths)
set WGPU_BACKEND=vulkan          # Options: vulkan, dx12, metal, gl, webgpu
set WGPU_BACKEND=dx12            # Force DirectX 12 on Windows
set WGPU_BACKEND=gl              # Force OpenGL fallback

# Power preference for adapter selection
set WGPU_POWER_PREF=high         # Use discrete/dedicated GPU
set WGPU_POWER_PREF=low          # Use integrated GPU
set WGPU_POWER_PREF=none         # Default preference

# Adapter name filtering (partial match on adapter name)
set WGPU_ADAPTER_NAME=NVIDIA     # Select NVIDIA GPU
set WGPU_ADAPTER_NAME=Intel      # Select Intel integrated graphics

# Enable WGPU tracing to file (generates trace for debugging)
set WGPU_TRACE=wgpu_trace.json   # Record all WGPU commands

# RUST_LOG levels for WGPU
set RUST_LOG=wgpu=debug          # Debug level WGPU logging
set RUST_LOG=wgpu_core=info      # Core WGPU operations
set RUST_LOG=wgpu_hal=debug      # Hardware abstraction layer
```

### 2.2 Adapter Enumeration and Selection

Bevy 0.13.2 uses WGPU 0.19's adapter selection. Here's how to diagnose adapter issues:

#### Adapter Info Structure

```rust
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapter;
use bevy::render::RenderApp;

/// System to log adapter information
pub fn log_adapter_info(render_adapter: Res<RenderAdapter>) {
    let info = render_adapter.get_info();
    
    log::info!("[WGPU ADAPTER] Backend: {:?}", info.backend);
    log::info!("[WGPU ADAPTER] Name: {}", info.name);
    log::info!("[WGPU ADAPTER] Vendor ID: 0x{:04X}", info.vendor);
    log::info!("[WGPU ADAPTER] Device ID: 0x{:04X}", info.device);
    log::info!("[WGPU ADAPTER] Adapter Type: {:?}", info.adapter_type);
    log::info!("[WGPU ADAPTER] Driver: {}", info.driver);
    log::info!("[WGPU ADAPTER] Driver Info: {}", info.driver_info);
}
```

#### Vendor ID Reference

| Vendor ID | Vendor Name | Common GPUs |
|-----------|-------------|-------------|
| 0x10DE | NVIDIA | GeForce RTX series, GTX series |
| 0x1002 | AMD | Radeon RX series, Ryzen integrated |
| 0x8086 | Intel | Arc, Iris Xe, HD Graphics |
| 0x13B5 | Apple | M1, M2, M3 series |
| 0x5143 | Qualcomm | Adreno (mobile) |
| 0x1AE0 | Google | SwiftShader (software) |

#### Backend Selection Priority

WGPU 0.19 backend priority on different platforms:

```rust
// Default backend selection order:
// Windows: Vulkan → DirectX 12 → OpenGL
// Linux:   Vulkan → OpenGL  
// macOS:   Metal
// Web:     WebGPU → WebGL2

use bevy::render::settings::Backends;

// In RenderPlugin configuration:
WgpuSettings {
    backends: Some(Backends::VULKAN),           // Force Vulkan
    // OR
    backends: Some(Backends::DX12),             // Force DirectX 12
    // OR
    backends: Some(Backends::all()),            // All backends (default)
    // OR
    backends: Some(Backends::VULKAN | Backends::DX12), // Vulkan or DX12 only
    ..Default::default()
}
```

### 2.3 Adapter Features Validation

Different GPUs support different WGPU features. Missing required features cause runtime failures.

```rust
use bevy::render::renderer::RenderAdapter;
use wgpu::Features;

/// Critical features for Rose Online rendering
pub fn validate_adapter_features(render_adapter: Res<RenderAdapter>) {
    let features = render_adapter.features();
    
    // Check for texture array support (required for zone rendering)
    let texture_arrays = features.contains(Features::TEXTURE_BINDING_ARRAY);
    log::info!("[WGPU FEATURE] TEXTURE_BINDING_ARRAY: {}", texture_arrays);
    
    // Check for push constants (useful for shader performance)
    let push_constants = features.contains(Features::PUSH_CONSTANTS);
    log::info!("[WGPU FEATURE] PUSH_CONSTANTS: {}", push_constants);
    
    // Check for anisotropic filtering
    let anisotropic = features.contains(Features::SAMPLER_ANISOTROPY);
    log::info!("[WGPU FEATURE] SAMPLER_ANISOTROPY: {}", anisotropic);
    
    // Check for storage buffers in vertex shaders
    let storage_vs = features.contains(Features::VERTEX_WRITABLE_STORAGE);
    log::info!("[WGPU FEATURE] VERTEX_WRITABLE_STORAGE: {}", storage_vs);
    
    // Check for mappable primary buffers (faster CPU-GPU transfer)
    let mappable = features.contains(Features::MAPPABLE_PRIMARY_BUFFERS);
    log::info!("[WGPU FEATURE] MAPPABLE_PRIMARY_BUFFERS: {}", mappable);
    
    // Required features validation
    if !texture_arrays {
        log::error!("[WGPU FEATURE ERROR] TEXTURE_BINDING_ARRAY not supported! Zone rendering may fail.");
    }
}
```

#### Feature Flag Reference

| Feature | Required For | Fallback Strategy |
|---------|-------------|-------------------|
| `TEXTURE_BINDING_ARRAY` | Zone texture arrays, material bindless | Use texture atlases |
| `PUSH_CONSTANTS` | Fast per-draw shader data | Use uniform buffers |
| `SAMPLER_ANISOTROPY` | High-quality texture filtering | Use linear filtering |
| `VERTEX_WRITABLE_STORAGE` | GPU particle systems | CPU particle updates |
| `MAPPABLE_PRIMARY_BUFFERS` | Fast mesh upload | Use staging buffers |

### 2.4 Adapter Limits Validation

GPU limits determine maximum resource sizes. Exceeding limits causes allocation failures.

```rust
use bevy::render::renderer::RenderAdapter;

/// Validate adapter limits against Rose Online requirements
pub fn validate_adapter_limits(render_adapter: Res<RenderAdapter>) {
    let limits = &render_adapter.limits();
    
    // Texture size limits
    log::info!("[WGPU LIMIT] Max Texture Dimension 2D: {}", limits.max_texture_dimension_2d);
    log::info!("[WGPU LIMIT] Max Texture Dimension 3D: {}", limits.max_texture_dimension_3d);
    log::info!("[WGPU LIMIT] Max Texture Array Layers: {}", limits.max_texture_array_layers);
    
    // Buffer limits
    log::info!("[WGPU LIMIT] Max Uniform Buffer Binding Size: {} bytes ({} MB)", 
        limits.max_uniform_buffer_binding_size,
        limits.max_uniform_buffer_binding_size / (1024 * 1024));
    log::info!("[WGPU LIMIT] Max Storage Buffer Binding Size: {} bytes", 
        limits.max_storage_buffer_binding_size);
    log::info!("[WGPU LIMIT] Max Buffer Size: {} bytes", limits.max_buffer_size);
    
    // Binding limits
    log::info!("[WGPU LIMIT] Max Bind Groups: {}", limits.max_bind_groups);
    log::info!("[WGPU LIMIT] Max Bindings Per Bind Group: {}", limits.max_bindings_per_bind_group);
    log::info!("[WGPU LIMIT] Max Sampled Textures Per Shader Stage: {}", 
        limits.max_sampled_textures_per_shader_stage);
    log::info!("[WGPU LIMIT] Max Samplers Per Shader Stage: {}", 
        limits.max_samplers_per_shader_stage);
    
    // Compute limits (if using compute shaders)
    log::info!("[WGPU LIMIT] Max Compute Workgroup Size: {:?}", 
        limits.max_compute_workgroup_size);
    log::info!("[WGPU LIMIT] Max Compute Invocations Per Workgroup: {}", 
        limits.max_compute_invocations_per_workgroup);
    
    // Validation against Rose Online requirements
    let min_texture_size: u32 = 4096;  // 4K textures
    let min_uniform_buffer: u32 = 64 * 1024; // 64KB for scene data
    
    if limits.max_texture_dimension_2d < min_texture_size {
        log::warn!("[WGPU LIMIT WARNING] Max texture size {} < required {}", 
            limits.max_texture_dimension_2d, min_texture_size);
    }
    
    if limits.max_uniform_buffer_binding_size < min_uniform_buffer {
        log::warn!("[WGPU LIMIT WARNING] Uniform buffer limit {} < required {}", 
            limits.max_uniform_buffer_binding_size, min_uniform_buffer);
    }
}
```

### 2.5 Power Preference Configuration

```rust
use bevy::render::settings::WgpuSettings;
use bevy::render::render_resource::PowerPreference;

// Configure power preference in RenderPlugin
WgpuSettings {
    power_preference: PowerPreference::HighPerformance,  // Discrete GPU
    // OR
    power_preference: PowerPreference::LowPower,         // Integrated GPU
    ..Default::default()
}
```

---

## 3. Device and Queue Configuration

### 3.1 Device Descriptor Validation

The device request specifies required features and limits. Failure to match adapter capabilities causes device creation to fail.

```rust
use bevy::render::renderer::RenderDevice;
use bevy::render::RenderApp;

/// Log device capabilities after creation
pub fn log_device_info(render_device: Res<RenderDevice>) {
    // Device was successfully created if we can access it
    log::info!("[WGPU DEVICE] Device successfully created");
    
    // Query enabled features on device
    let enabled_features = render_device.features();
    log::info!("[WGPU DEVICE] Enabled features: {:?}", enabled_features);
    
    // Query actual limits enforced on device
    let limits = render_device.limits();
    log::info!("[WGPU DEVICE] Enforced limits validated");
}
```

### 3.2 Queue Submission Timing Diagnostics

```rust
use bevy::render::renderer::RenderQueue;
use std::time::Instant;

/// Diagnostic system to monitor queue submission timing
pub fn monitor_queue_submissions(
    render_queue: Res<RenderQueue>,
    mut last_submission: Local<Option<Instant>>,
) {
    let now = Instant::now();
    
    if let Some(last) = *last_submission {
        let elapsed = now.duration_since(last);
        if elapsed.as_millis() > 100 {
            log::warn!("[WGPU QUEUE] Long gap between submissions: {:?}", elapsed);
        }
    }
    
    *last_submission = Some(now);
    
    // Note: Actual submission happens during render graph execution
    // This system just tracks timing from Bevy's perspective
}
```

### 3.3 Command Encoder Lifecycle

Understanding command encoder creation and submission:

```rust
use bevy::render::renderer::RenderDevice;
use wgpu::CommandEncoderDescriptor;

/// Example of creating and finishing a command encoder
pub fn create_command_encoder_example(render_device: &RenderDevice) {
    // Create command encoder
    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Diagnostic Encoder"),
    });
    
    // Commands would be recorded here...
    
    // Finish encoder to get command buffer
    let command_buffer = encoder.finish();
    
    // Command buffer is then submitted to queue
    // render_queue.submit(std::iter::once(command_buffer));
}
```

### 3.4 Device Lost Event Handling

Device loss occurs when the GPU driver crashes, is reset, or the GPU is removed.

```rust
use bevy::prelude::*;
use bevy::render::renderer::RenderDevice;

/// Plugin to setup device lost callback (requires render world access)
pub struct DeviceLostDiagnosticPlugin;

impl Plugin for DeviceLostDiagnosticPlugin {
    fn build(&self, app: &mut App) {
        // Note: Device lost callback must be set during device creation
        // Bevy 0.13.2 doesn't expose a direct hook, but we can monitor for symptoms
        
        app.add_systems(Update, monitor_for_device_loss);
    }
}

/// Monitor for symptoms of device loss
fn monitor_for_device_loss(
    // Query for resources that would indicate device loss
    render_device: Option<Res<RenderDevice>>,
) {
    // If RenderDevice is missing unexpectedly, device may be lost
    if render_device.is_none() {
        log::error!("[WGPU DEVICE LOST] RenderDevice resource missing - possible device loss!");
        log::error!("[WGPU DEVICE LOST] Symptoms: Black screen, frozen rendering, DXGI_ERROR_DEVICE_REMOVED");
    }
}

/// Raw WGPU device lost callback setup (if creating device manually)
#[cfg(feature = "manual-device-creation")]
pub fn setup_device_lost_callback() {
    // This would be done in a custom RenderPlugin
    // WGPU 0.19 device lost callback:
    /*
    let device_lost_callback = DeviceLostCallback::from(|info| {
        log::error!("[WGPU DEVICE LOST] Reason: {:?}", info.reason);
        log::error!("[WGPU DEVICE LOST] Message: {}", info.message);
        
        match info.reason {
            DeviceLostReason::Unknown => {
                log::error!("[WGPU DEVICE LOST] Unknown cause - check GPU drivers");
            }
            DeviceLostReason::Destroyed => {
                log::info!("[WGPU DEVICE LOST] Device intentionally destroyed");
            }
            DeviceLostReason::Dropped => {
                log::warn!("[WGPU DEVICE LOST] Device dropped due to inactivity");
            }
        }
    });
    */
}
```

#### Common Device Loss Causes

| Symptom | Likely Cause | Resolution |
|---------|--------------|------------|
| `DXGI_ERROR_DEVICE_REMOVED` | GPU timeout (TDR), driver crash | Reduce shader complexity, update drivers |
| `DXGI_ERROR_DEVICE_RESET` | GPU reset by OS/driver | Check for overheating, reduce GPU load |
| Black screen + frozen input | Device lost without error | Monitor device lost callback |
| `OutOfMemory` errors | GPU memory exhaustion | Reduce texture resolution, close other apps |

---

## 4. Surface Configuration Diagnostics

### 4.1 Surface Capabilities Querying

Before configuring the surface, query its capabilities to determine valid configurations.

```rust
use bevy::window::Window;
use bevy::render::view::ExtractedWindows;
use wgpu::{Surface, SurfaceCapabilities};

/// Log surface capabilities for debugging
pub fn log_surface_capabilities(
    windows: Query<&Window>,
) {
    for window in &windows {
        let window_id = window.id();
        
        // Note: Surface capabilities are queried internally by WGPU
        // We can access them through the render world in Bevy
        log::info!("[WGPU SURFACE] Window {:?} surface capabilities query", window_id);
        
        // Log window properties that affect surface config
        log::info!("[WGPU SURFACE] Window size: {}x{}", 
            window.resolution.width(), 
            window.resolution.height());
        log::info!("[WGPU SURFACE] Window present mode: {:?}", window.present_mode);
    }
}

/// Extracted windows diagnostics (runs in render world)
pub fn log_extracted_window_info(
    extracted_windows: Res<ExtractedWindows>,
) {
    for (id, window) in extracted_windows.iter() {
        log::info!("[WGPU SURFACE] Extracted window {:?}: {}x{}", 
            id, window.physical_width, window.physical_height);
        log::info!("[WGPU SURFACE] Present mode: {:?}", window.present_mode);
    }
}
```

### 4.2 Presentation Mode Selection

Present modes control vsync and buffering behavior:

```rust
use bevy::window::PresentMode;

/// Present mode selection diagnostics
pub fn diagnose_present_mode(mode: PresentMode) {
    match mode {
        PresentMode::Fifo => {
            log::info!("[WGPU PRESENT] Fifo mode (vsync enabled, triple buffering)");
            log::info!("[WGPU PRESENT] - Frames queued in FIFO order");
            log::info!("[WGPU PRESENT] - Latency: ~1-3 frames");
            log::info!("[WGPU PRESENT] - Power efficient, no tearing");
        }
        PresentMode::Immediate => {
            log::info!("[WGPU PRESENT] Immediate mode (vsync disabled)");
            log::info!("[WGPU PRESENT] - Frames displayed immediately");
            log::info!("[WGPU PRESENT] - Lowest latency");
            log::info!("[WGPU PRESENT] - May cause tearing on non-VRR displays");
        }
        PresentMode::Mailbox => {
            log::info!("[WGPU PRESENT] Mailbox mode (single-buffered vsync)");
            log::info!("[WGPU PRESENT] - Latest frame replaces pending frame");
            log::info!("[WGPU PRESENT] - Low latency with vsync");
            log::info!("[WGPU PRESENT] - Requires hardware support");
        }
        PresentMode::FifoRelaxed => {
            log::info!("[WGPU PRESENT] FifoRelaxed mode (adaptive vsync)");
            log::info!("[WGPU PRESENT] - Tears when below refresh rate");
            log::info!("[WGPU PRESENT] - Smooth when above refresh rate");
        }
        _ => {
            log::warn!("[WGPU PRESENT] Unknown present mode");
        }
    }
}

/// Configure present mode in WindowPlugin
pub fn configure_present_mode() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            present_mode: PresentMode::Fifo,  // Default: vsync on
            // present_mode: PresentMode::Immediate,  // For testing: no vsync
            ..Default::default()
        }),
        ..Default::default()
    }
}
```

### 4.3 Format Validation

Surface format determines how pixel data is stored and interpreted.

```rust
use wgpu::TextureFormat;

/// Surface format diagnostics
pub fn diagnose_surface_format(format: TextureFormat) {
    match format {
        TextureFormat::Bgra8Unorm => {
            log::info!("[WGPU FORMAT] Bgra8Unorm - 8-bit BGRA, no SRGB");
            log::info!("[WGPU FORMAT] - Linear color space");
            log::info!("[WGPU FORMAT] - Requires manual gamma correction");
        }
        TextureFormat::Bgra8UnormSrgb => {
            log::info!("[WGPU FORMAT] Bgra8UnormSrgb - 8-bit BGRA, SRGB");
            log::info!("[WGPU FORMAT] - Automatic SRGB conversion");
            log::info!("[WGPU FORMAT] - Recommended for standard rendering");
        }
        TextureFormat::Rgba8Unorm => {
            log::info!("[WGPU FORMAT] Rgba8Unorm - 8-bit RGBA, no SRGB");
        }
        TextureFormat::Rgba8UnormSrgb => {
            log::info!("[WGPU FORMAT] Rgba8UnormSrgb - 8-bit RGBA, SRGB");
        }
        TextureFormat::Rgba16Float => {
            log::info!("[WGPU FORMAT] Rgba16Float - 16-bit float HDR");
            log::info!("[WGPU FORMAT] - For HDR rendering pipelines");
        }
        _ => {
            log::info!("[WGPU FORMAT] Other format: {:?}", format);
        }
    }
}

/// Check if format is SRGB (affects color correctness)
pub fn is_srgb_format(format: TextureFormat) -> bool {
    matches!(format,
        TextureFormat::Bgra8UnormSrgb |
        TextureFormat::Rgba8UnormSrgb |
        TextureFormat::Rgb10a2Unorm |
        TextureFormat::Rgba16Float
    )
}
```

### 4.4 Alpha Mode Configuration

```rust
use wgpu::CompositeAlphaMode;

/// Alpha mode diagnostics
pub fn diagnose_alpha_mode(mode: CompositeAlphaMode) {
    match mode {
        CompositeAlphaMode::Auto => {
            log::info!("[WGPU ALPHA] Auto - let WGPU choose best mode");
        }
        CompositeAlphaMode::Opaque => {
            log::info!("[WGPU ALPHA] Opaque - ignore alpha channel");
            log::info!("[WGPU ALPHA] - Best performance, no transparency");
        }
        CompositeAlphaMode::PreMultiplied => {
            log::info!("[WGPU ALPHA] PreMultiplied - pre-multiplied alpha");
            log::info!("[WGPU ALPHA] - For transparent windows/overlays");
        }
        CompositeAlphaMode::PostMultiplied => {
            log::info!("[WGPU ALPHA] PostMultiplied - post-multiplied alpha");
        }
        CompositeAlphaMode::Inherit => {
            log::info!("[WGPU ALPHA] Inherit - from parent/compositor");
        }
        _ => {
            log::info!("[WGPU ALPHA] Unknown alpha mode");
        }
    }
}
```

---

## 5. Swap Chain Health Monitoring

### 5.1 Surface Texture Acquisition Timeouts

```rust
use bevy::prelude::*;
use bevy::render::renderer::RenderDevice;
use std::time::{Duration, Instant};

/// Resource to track surface acquisition timing
#[derive(Resource, Default)]
pub struct SurfaceAcquisitionDiagnostics {
    pub last_successful_acquisition: Option<Instant>,
    pub acquisition_failures: u32,
    pub consecutive_failures: u32,
}

/// Monitor for surface acquisition issues
pub fn monitor_surface_acquisition(
    mut diagnostics: ResMut<SurfaceAcquisitionDiagnostics>,
) {
    let now = Instant::now();
    
    // Check for extended period without successful acquisition
    if let Some(last) = diagnostics.last_successful_acquisition {
        let elapsed = now.duration_since(last);
        if elapsed > Duration::from_secs(1) {
            log::error!("[WGPU SURFACE] No successful texture acquisition for {:?}", elapsed);
            log::error!("[WGPU SURFACE] Possible causes:");
            log::error!("[WGPU SURFACE] - Surface lost/outdated");
            log::error!("[WGPU SURFACE] - GPU driver hang");
            log::error!("[WGPU SURFACE] - Window minimized/invisible");
        }
    }
    
    if diagnostics.consecutive_failures > 10 {
        log::error!("[WGPU SURFACE] {} consecutive acquisition failures", 
            diagnostics.consecutive_failures);
    }
}

/// Mark successful acquisition (call from render submission)
pub fn mark_acquisition_success(mut diagnostics: ResMut<SurfaceAcquisitionDiagnostics>) {
    diagnostics.last_successful_acquisition = Some(Instant::now());
    diagnostics.consecutive_failures = 0;
}
```

### 5.2 Outdated Surface Detection and Recreation

```rust
use bevy::window::Window;
use bevy::render::view::ExtractedWindows;

/// Detect when surface needs recreation
pub fn detect_outdated_surface(
    windows: Query<&Window, Changed<Window>>,
) {
    for window in &windows {
        log::info!("[WGPU SURFACE] Window changed - checking for resize");
        log::info!("[WGPU SURFACE] New size: {}x{}", 
            window.resolution.width(), 
            window.resolution.height());
        
        // Surface recreation triggers automatically in Bevy 0.13.2
        // when window size changes
    }
}
```

### 5.3 Lost Surface Handling

```rust
use bevy::prelude::*;

/// System to handle surface loss scenarios
pub fn handle_surface_loss(
    // Query for window state
    windows: Query<&Window>,
) {
    for window in &windows {
        // Check for conditions that can cause surface loss
        if window.resolution.width() == 0.0 || window.resolution.height() == 0.0 {
            log::warn!("[WGPU SURFACE] Window has zero dimensions - surface may be lost");
        }
    }
}

/// Recovery steps for lost surface
pub fn surface_loss_recovery_guide() {
    log::info!("[WGPU SURFACE RECOVERY] Surface loss detected, attempting recovery...");
    log::info!("[WGPU SURFACE RECOVERY] 1. Check if window is minimized");
    log::info!("[WGPU SURFACE RECOVERY] 2. Check if display was disconnected");
    log::info!("[WGPU SURFACE RECOVERY] 3. Verify GPU driver is still running");
    log::info!("[WGPU SURFACE RECOVERY] 4. Trigger window resize to force recreation");
}
```

### 5.4 Suboptimal Presentation Detection

```rust
/// Monitor for suboptimal presentation conditions
pub fn detect_suboptimal_presentation(
    // Access to render world diagnostics
) {
    // Suboptimal presentation can occur when:
    // - Present mode doesn't match display refresh
    // - Surface format requires conversion
    // - Resolution doesn't match native display resolution
    
    log::info!("[WGPU PRESENTATION] Monitoring for suboptimal conditions...");
    log::info!("[WGPU PRESENTATION] Suboptimal indicators:");
    log::info!("[WGPU PRESENTATION] - Frame time variance > 2ms");
    log::info!("[WGPU PRESENTATION] - Regular frame drops");
    log::info!("[WGPU PRESENTATION] - Tearing artifacts");
}
```

---

## 6. Platform-Specific Backend Diagnostics

### 6.1 Vulkan Diagnostics

#### Validation Layer Setup

```powershell
# Enable Vulkan validation layers
set VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation

# Enable additional validation features
set VK_LAYER_ENABLES=VK_VALIDATION_FEATURE_ENABLE_BEST_PRACTICES_EXT

# Vulkan debug output
set VK_LOADER_DEBUG=all
```

#### Vulkan-Specific Diagnostics

```rust
#[cfg(target_os = "windows")]
pub mod vulkan_diagnostics {
    use bevy::log;
    
    /// Log Vulkan-specific adapter info
    pub fn log_vulkan_info(adapter_name: &str) {
        log::info!("[VULKAN] Backend: Vulkan");
        log::info!("[VULKAN] Adapter: {}", adapter_name);
        
        // Check for known problematic drivers
        if adapter_name.contains("Intel") {
            log::warn!("[VULKAN] Intel GPU detected - may have driver issues with some features");
        }
        
        // Environment variables that affect Vulkan
        log::info!("[VULKAN] Relevant environment variables:");
        log::info!("[VULKAN]   VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation");
        log::info!("[VULKAN]   VK_DEVICE_LAYERS=VK_LAYER_KHRONOS_validation");
        log::info!("[VULKAN]   MESA_VK_VERSION_OVERRIDE=1.3");
    }
    
    /// Common Vulkan errors and solutions
    pub fn diagnose_vulkan_error(error_msg: &str) {
        if error_msg.contains("VK_ERROR_OUT_OF_DEVICE_MEMORY") {
            log::error!("[VULKAN ERROR] Out of GPU memory");
            log::error!("[VULKAN ERROR] Solutions: Reduce texture quality, close other apps");
        } else if error_msg.contains("VK_ERROR_DEVICE_LOST") {
            log::error!("[VULKAN ERROR] GPU device lost (TDR timeout)");
            log::error!("[VULKAN ERROR] Solutions: Simplify shaders, reduce draw calls");
        } else if error_msg.contains("VK_ERROR_SURFACE_LOST_KHR") {
            log::error!("[VULKAN ERROR] Surface lost - window/display issue");
            log::error!("[VULKAN ERROR] Solutions: Check display connection, recreate window");
        }
    }
}
```

### 6.2 DirectX 12 Diagnostics

#### DXGI Debug Interface

```powershell
# Enable DirectX 12 debug layer
set DXGI_DEBUG=1

# Enable D3D12 debug
set D3D12_DEBUG=1

# Enable GPU-based validation (slow but thorough)
set D3D12_GPU_BASED_VALIDATION=1
```

#### DirectX 12 Device Removed Detection

```rust
#[cfg(target_os = "windows")]
pub mod dx12_diagnostics {
    use bevy::log;
    
    /// Log DirectX 12 specific information
    pub fn log_dx12_info(adapter_name: &str) {
        log::info!("[DX12] Backend: DirectX 12");
        log::info!("[DX12] Adapter: {}", adapter_name);
        
        // DirectX 12 feature levels
        log::info!("[DX12] Feature level detection requires DXGI query");
        
        // Known issues
        if adapter_name.contains("NVIDIA") {
            log::info!("[DX12] NVIDIA GPU - ensure drivers are 511.xx or newer");
        }
    }
    
    /// DXGI error codes and meanings
    pub fn diagnose_dxgi_error(error_code: i32) {
        match error_code {
            0x887A0005 => { // DXGI_ERROR_DEVICE_REMOVED
                log::error!("[DX12 ERROR] DXGI_ERROR_DEVICE_REMOVED");
                log::error!("[DX12 ERROR] GPU was physically removed or driver crashed");
                log::error!("[DX12 ERROR] Check Windows Event Viewer for TDR events");
            }
            0x887A0006 => { // DXGI_ERROR_DEVICE_HUNG
                log::error!("[DX12 ERROR] DXGI_ERROR_DEVICE_HUNG");
                log::error!("[DX12 ERROR] GPU stopped responding (timeout)");
                log::error!("[DX12 ERROR] Reduce shader complexity or draw call count");
            }
            0x887A0007 => { // DXGI_ERROR_DEVICE_RESET
                log::error!("[DX12 ERROR] DXGI_ERROR_DEVICE_RESET");
                log::error!("[DX12 ERROR] GPU was reset by OS");
            }
            0x887A0020 => { // DXGI_ERROR_DRIVER_INTERNAL_ERROR
                log::error!("[DX12 ERROR] DXGI_ERROR_DRIVER_INTERNAL_ERROR");
                log::error!("[DX12 ERROR] Driver bug encountered");
                log::error!("[DX12 ERROR] Try updating GPU drivers");
            }
            _ => {
                log::error!("[DX12 ERROR] Unknown DXGI error: 0x{:08X}", error_code);
            }
        }
    }
    
    /// Enable DirectX debug layer programmatically
    pub fn enable_dx12_debug_layer() {
        log::info!("[DX12] To enable debug layer, set environment variables:");
        log::info!("[DX12]   DXGIDebug=1");
        log::info!("[DX12]   D3D12_DEBUG=1");
        log::info!("[DX12]   D3D12_GPU_BASED_VALIDATION=1");
    }
}
```

### 6.3 Metal Diagnostics

#### Metal Validation

```bash
# Enable Metal validation (macOS)
export METAL_DEVICE_WRAPPER_TYPE=1
export MTL_DEBUG_LAYER=1

# Enable Metal API validation
export METAL_DEBUG_ERROR_MODE=3
```

#### Metal-Specific Diagnostics

```rust
#[cfg(target_os = "macos")]
pub mod metal_diagnostics {
    use bevy::log;
    
    /// Log Metal-specific information
    pub fn log_metal_info(adapter_name: &str) {
        log::info!("[METAL] Backend: Metal");
        log::info!("[METAL] Device: {}", adapter_name);
        
        // Metal version detection
        if adapter_name.contains("Apple M") {
            log::info!("[METAL] Apple Silicon detected");
            log::info!("[METAL] Unified memory architecture");
        } else if adapter_name.contains("Intel") {
            log::warn!("[METAL] Intel GPU on macOS - limited feature support");
        } else if adapter_name.contains("AMD") {
            log::info!("[METAL] AMD GPU detected");
        }
    }
    
    /// Metal validation setup
    pub fn setup_metal_validation() {
        log::info!("[METAL] Enable validation with:");
        log::info!("[METAL]   METAL_DEVICE_WRAPPER_TYPE=1");
        log::info!("[METAL]   MTL_DEBUG_LAYER=1");
        log::info!("[METAL] Run from Xcode for GPU frame capture");
    }
}
```

### 6.4 OpenGL Fallback Diagnostics

#### Context Creation

```rust
#[cfg(all(target_os = "windows", feature = "opengl"))]
pub mod opengl_diagnostics {
    use bevy::log;
    
    /// Log OpenGL fallback information
    pub fn log_opengl_info(adapter_name: &str) {
        log::warn!("[OPENGL] Backend: OpenGL (fallback mode)");
        log::warn!("[OPENGL] Adapter: {}", adapter_name);
        log::warn!("[OPENGL] Performance may be reduced compared to Vulkan/DX12");
        
        // OpenGL version requirements
        log::info!("[OPENGL] Minimum required: OpenGL 3.3 Core Profile");
        log::info!("[OPENGL] Recommended: OpenGL 4.5 or newer");
    }
    
    /// Extension validation
    pub fn validate_opengl_extensions(extensions: &[String]) {
        let required = vec![
            "GL_ARB_buffer_storage",
            "GL_ARB_vertex_array_object",
            "GL_ARB_framebuffer_object",
        ];
        
        for ext in required {
            if extensions.iter().any(|e| e.contains(ext)) {
                log::info!("[OPENGL] Extension available: {}", ext);
            } else {
                log::error!("[OPENGL] Missing required extension: {}", ext);
            }
        }
    }
    
    /// Common OpenGL errors
    pub fn diagnose_opengl_error(error_msg: &str) {
        if error_msg.contains("GL_OUT_OF_MEMORY") {
            log::error!("[OPENGL ERROR] Out of memory");
            log::error!("[OPENGL ERROR] Solutions: Reduce texture quality, use texture compression");
        } else if error_msg.contains("GL_CONTEXT_LOST") {
            log::error!("[OPENGL ERROR] Context lost");
            log::error!("[OPENGL ERROR] Driver or system issue");
        }
    }
}
```

---

## 7. Diagnostic Code Snippets

### 7.1 Complete Adapter Logging System

```rust
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapter;
use bevy::render::RenderApp;
use wgpu::{Features, Limits};

/// Plugin to add comprehensive adapter diagnostics
pub struct AdapterDiagnosticPlugin;

impl Plugin for AdapterDiagnosticPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, log_full_adapter_info);
        app.add_systems(Startup, validate_rose_online_requirements);
    }
}

/// Comprehensive adapter information logging
fn log_full_adapter_info(render_adapter: Res<RenderAdapter>) {
    let info = render_adapter.get_info();
    let features = render_adapter.features();
    let limits = &render_adapter.limits();
    
    log::info!("========================================");
    log::info!("WGPU ADAPTER DIAGNOSTICS");
    log::info!("========================================");
    
    // Basic info
    log::info!("Backend:               {:?}", info.backend);
    log::info!("Name:                  {}", info.name);
    log::info!("Vendor ID:             0x{:04X} ({})", info.vendor, vendor_name(info.vendor));
    log::info!("Device ID:             0x{:04X}", info.device);
    log::info!("Adapter Type:          {:?}", info.adapter_type);
    log::info!("Driver:                {}", info.driver);
    log::info!("Driver Info:           {}", info.driver_info);
    
    // Features
    log::info!("");
    log::info!("FEATURES:");
    log_feature_status(features, Features::TEXTURE_BINDING_ARRAY, "TEXTURE_BINDING_ARRAY");
    log_feature_status(features, Features::PUSH_CONSTANTS, "PUSH_CONSTANTS");
    log_feature_status(features, Features::SAMPLER_ANISOTROPY, "SAMPLER_ANISOTROPY");
    log_feature_status(features, Features::VERTEX_WRITABLE_STORAGE, "VERTEX_WRITABLE_STORAGE");
    log_feature_status(features, Features::MAPPABLE_PRIMARY_BUFFERS, "MAPPABLE_PRIMARY_BUFFERS");
    log_feature_status(features, Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES, "TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES");
    log_feature_status(features, Features::MULTI_DRAW_INDIRECT, "MULTI_DRAW_INDIRECT");
    log_feature_status(features, Features::INDIRECT_FIRST_INSTANCE, "INDIRECT_FIRST_INSTANCE");
    
    // Limits
    log::info!("");
    log::info!("LIMITS:");
    log::info!("  Max Texture 2D:      {}", limits.max_texture_dimension_2d);
    log::info!("  Max Texture 3D:      {}", limits.max_texture_dimension_3d);
    log::info!("  Max Texture Layers:  {}", limits.max_texture_array_layers);
    log::info!("  Max Uniform Buffer:  {} bytes", limits.max_uniform_buffer_binding_size);
    log::info!("  Max Storage Buffer:  {} bytes", limits.max_storage_buffer_binding_size);
    log::info!("  Max Buffer Size:     {} bytes", limits.max_buffer_size);
    log::info!("  Max Bind Groups:     {}", limits.max_bind_groups);
    log::info!("  Max Vertex Buffers:  {}", limits.max_vertex_buffers);
    log::info!("  Max Vertex Attributes: {}", limits.max_vertex_attributes);
    log::info!("========================================");
}

fn log_feature_status(features: Features, flag: Features, name: &str) {
    let status = if features.contains(flag) { "✓ ENABLED" } else { "✗ DISABLED" };
    log::info!("  {}: {}", name, status);
}

fn vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x10DE => "NVIDIA",
        0x1002 => "AMD",
        0x8086 => "Intel",
        0x13B5 => "Apple",
        0x5143 => "Qualcomm",
        0x1AE0 => "Google (SwiftShader)",
        _ => "Unknown",
    }
}

/// Validate Rose Online specific requirements
fn validate_rose_online_requirements(
    render_adapter: Res<RenderAdapter>,
) {
    let features = render_adapter.features();
    let limits = &render_adapter.limits();
    
    log::info!("========================================");
    log::info!("ROSE ONLINE REQUIREMENTS VALIDATION");
    log::info!("========================================");
    
    let mut all_passed = true;
    
    // Check texture size for zone textures
    if limits.max_texture_dimension_2d >= 2048 {
        log::info!("✓ Texture size (2048+): PASS ({})", limits.max_texture_dimension_2d);
    } else {
        log::error!("✗ Texture size (2048+): FAIL ({})", limits.max_texture_dimension_2d);
        all_passed = false;
    }
    
    // Check uniform buffer size for scene data
    if limits.max_uniform_buffer_binding_size >= 65536 {
        log::info!("✓ Uniform buffer (64KB+): PASS ({} bytes)", 
            limits.max_uniform_buffer_binding_size);
    } else {
        log::warn!("⚠ Uniform buffer (64KB+): LOW ({} bytes)", 
            limits.max_uniform_buffer_binding_size);
    }
    
    // Check texture array support
    if features.contains(Features::TEXTURE_BINDING_ARRAY) {
        log::info!("✓ TEXTURE_BINDING_ARRAY: PASS");
    } else {
        log::warn!("⚠ TEXTURE_BINDING_ARRAY: MISSING - Zone rendering may be slower");
    }
    
    // Check anisotropic filtering
    if features.contains(Features::SAMPLER_ANISOTROPY) {
        log::info!("✓ SAMPLER_ANISOTROPY: PASS");
    } else {
        log::warn!("⚠ SAMPLER_ANISOTROPY: MISSING - Texture quality reduced");
    }
    
    if all_passed {
        log::info!("✓ All critical requirements met");
    } else {
        log::error!("✗ Some critical requirements failed - rendering may not work correctly");
    }
    log::info!("========================================");
}
```

### 7.2 Surface Capability Dumping

```rust
use bevy::prelude::*;
use bevy::window::Window;
use bevy::render::view::ExtractedWindows;
use wgpu::{PresentMode, TextureFormat, CompositeAlphaMode};

/// Plugin for surface diagnostics
pub struct SurfaceDiagnosticPlugin;

impl Plugin for SurfaceDiagnosticPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, dump_surface_configuration);
        app.add_systems(Update, monitor_surface_health);
    }
}

/// Dump surface configuration info
fn dump_surface_configuration(
    windows: Query<&Window>,
) {
    for window in &windows {
        log::info!("========================================");
        log::info!("SURFACE CONFIGURATION");
        log::info!("========================================");
        
        log::info!("Window ID:          {:?}", window.id());
        log::info!("Physical Size:      {}x{}", 
            window.resolution.physical_width(), 
            window.resolution.physical_height());
        log::info!("Scale Factor:       {}", window.resolution.scale_factor());
        log::info!("Present Mode:       {:?}", window.present_mode);
        log::info!("Resizing:           {}", window.resizable);
        log::info!("Focused:            {}", window.focused);
        log::info!("Visible:            {}", window.visible);
        
        // Present mode explanation
        match window.present_mode {
            PresentMode::Fifo => {
                log::info!("  → VSync enabled, triple-buffered");
            }
            PresentMode::Immediate => {
                log::info!("  → VSync disabled, immediate presentation");
            }
            PresentMode::Mailbox => {
                log::info!("  → Low-latency vsync (single buffer)");
            }
            _ => {}
        }
        
        log::info!("========================================");
    }
}

/// Monitor surface health
fn monitor_surface_health(
    windows: Query<&Window>,
    time: Res<Time>,
    mut last_check: Local<f32>,
) {
    // Check every 5 seconds
    if time.elapsed_seconds() - *last_check < 5.0 {
        return;
    }
    *last_check = time.elapsed_seconds();
    
    for window in &windows {
        // Check for problematic states
        if window.resolution.physical_width() == 0 || window.resolution.physical_height() == 0 {
            log::error!("[SURFACE] Window has zero dimensions - surface may be invalid!");
        }
        
        if !window.visible {
            log::warn!("[SURFACE] Window is not visible - presentation paused");
        }
        
        if !window.focused {
            // This is informational only
            log::debug!("[SURFACE] Window not focused");
        }
    }
}
```

### 7.3 Frame Timing Diagnostics

```rust
use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

/// Resource for frame timing tracking
#[derive(Resource, Default)]
pub struct FrameTimingDiagnostics {
    pub frame_times: Vec<f32>,
    pub max_samples: usize,
    pub present_time_threshold: f32, // ms
}

impl FrameTimingDiagnostics {
    pub fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(120),
            max_samples: 120,
            present_time_threshold: 20.0, // 20ms = 50fps threshold
        }
    }
    
    pub fn add_frame_time(&mut self, time_ms: f32) {
        self.frame_times.push(time_ms);
        if self.frame_times.len() > self.max_samples {
            self.frame_times.remove(0);
        }
    }
    
    pub fn average(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }
    
    pub fn variance(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let avg = self.average();
        let variance = self.frame_times.iter()
            .map(|&x| (x - avg).powi(2))
            .sum::<f32>() / self.frame_times.len() as f32;
        variance.sqrt()
    }
}

/// System to track frame timing
pub fn frame_timing_diagnostics(
    diagnostics: Res<DiagnosticsStore>,
    mut timing: ResMut<FrameTimingDiagnostics>,
    time: Res<Time>,
    mut last_log: Local<f32>,
) {
    // Get frame time from Bevy diagnostics
    if let Some(frame_time) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        if let Some(&current) = frame_time.measurements().last() {
            let time_ms = current as f32;
            timing.add_frame_time(time_ms);
            
            // Check for frame time spikes
            if time_ms > timing.present_time_threshold {
                log::warn!("[FRAME TIMING] Frame spike detected: {:.2}ms (threshold: {:.2}ms)", 
                    time_ms, timing.present_time_threshold);
            }
        }
    }
    
    // Log summary every 10 seconds
    let now = time.elapsed_seconds();
    if now - *last_log > 10.0 {
        *last_log = now;
        
        let avg = timing.average();
        let var = timing.variance();
        let fps = if avg > 0.0 { 1000.0 / avg } else { 0.0 };
        
        log::info!("[FRAME TIMING] Last {} frames:", timing.frame_times.len());
        log::info!("  Average:  {:.2}ms ({:.1} FPS)", avg, fps);
        log::info!("  Variance: {:.2}ms", var);
        log::info!("  Target:   16.67ms (60 FPS)");
        
        if var > 5.0 {
            log::warn!("[FRAME TIMING] High frame time variance detected - possible stuttering");
        }
    }
}
```

### 7.4 Complete Diagnostic Suite Integration

```rust
use bevy::prelude::*;

/// Complete WGPU diagnostic plugin for Rose Online
pub struct WgpuDiagnosticPlugin;

impl Plugin for WgpuDiagnosticPlugin {
    fn build(&self, app: &mut App) {
        // Add all diagnostic plugins
        app.add_plugins(AdapterDiagnosticPlugin);
        app.add_plugins(SurfaceDiagnosticPlugin);
        
        // Add frame timing
        app.insert_resource(FrameTimingDiagnostics::new());
        app.add_systems(Update, frame_timing_diagnostics);
        
        // Add device loss monitoring
        app.insert_resource(SurfaceAcquisitionDiagnostics::default());
        app.add_systems(Update, (
            monitor_surface_acquisition,
            handle_surface_loss,
        ));
        
        log::info!("[WGPU DIAGNOSTICS] Full diagnostic suite initialized");
    }
}

/// Usage in main.rs or lib.rs:
/// 
/// ```rust
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(WgpuDiagnosticPlugin)
///     // ... rest of app
///     .run();
/// ```
```

### 7.5 Environment Variable Helper

```rust
use bevy::prelude::*;
use std::env;

/// System to log all WGPU-related environment variables
pub fn log_wgpu_environment() {
    log::info!("========================================");
    log::info!("WGPU ENVIRONMENT VARIABLES");
    log::info!("========================================");
    
    let vars = vec![
        "WGPU_BACKEND",
        "WGPU_POWER_PREF",
        "WGPU_ADAPTER_NAME",
        "WGPU_TRACE",
        "VK_INSTANCE_LAYERS",
        "VK_DEVICE_LAYERS",
        "DXGI_DEBUG",
        "D3D12_DEBUG",
        "METAL_DEVICE_WRAPPER_TYPE",
        "MTL_DEBUG_LAYER",
        "RUST_LOG",
    ];
    
    for var in &vars {
        match env::var(var) {
            Ok(value) => {
                log::info!("{}={}", var, value);
            }
            Err(_) => {
                log::info!("{} [not set]", var);
            }
        }
    }
    
    log::info!("========================================");
}

/// Set recommended environment variables for debugging
pub fn print_recommended_env_vars() {
    log::info!("Recommended environment variables for debugging:");
    log::info!("  Windows:");
    log::info!("    set WGPU_BACKEND=vulkan");
    log::info!("    set WGPU_POWER_PREF=high");
    log::info!("    set RUST_LOG=wgpu=debug,bevy_render=debug");
    log::info!("    set VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation");
    log::info!("");
    log::info!("  Linux:");
    log::info!("    export WGPU_BACKEND=vulkan");
    log::info!("    export WGPU_POWER_PREF=high");
    log::info!("    export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation");
    log::info!("");
    log::info!("  macOS:");
    log::info!("    export METAL_DEVICE_WRAPPER_TYPE=1");
    log::info!("    export MTL_DEBUG_LAYER=1");
}
```

---

## 8. Troubleshooting Guide

### 8.1 No Suitable GPU Found

**Symptoms**: 
- Panic: "No suitable GPU adapters found"
- Black screen immediately on startup

**Diagnostic Steps**:
1. Check environment variables:
   ```powershell
   echo %WGPU_BACKEND%
   echo %WGPU_ADAPTER_NAME%
   ```

2. Try forcing different backends:
   ```powershell
   set WGPU_BACKEND=dx12
   cargo run
   ```

3. Check GPU driver status:
   - Device Manager → Display adapters
   - Look for warning icons or missing drivers

**Resolutions**:
- Install/update GPU drivers
- Try OpenGL fallback: `set WGPU_BACKEND=gl`
- Verify GPU meets minimum requirements (DirectX 11 Feature Level 11_0 or Vulkan 1.1)

### 8.2 Surface Creation Failure

**Symptoms**:
- Error: `CreateSurfaceError`
- Window appears but remains black

**Diagnostic Steps**:
1. Check window handle validity
2. Verify display is connected and enabled
3. Check for multiple monitor issues

**Resolutions**:
- Force windowed mode initially
- Try different present mode: `PresentMode::Immediate`
- Check display scaling settings

### 8.3 DXGI_ERROR_DEVICE_REMOVED

**Symptoms**:
- Error code 0x887A0005
- Rendering freezes then black screen

**Diagnostic Steps**:
1. Check Windows Event Viewer for TDR events
2. Monitor GPU temperature
3. Check for shader compilation errors

**Resolutions**:
- Update GPU drivers
- Reduce GPU workload (fewer draw calls, simpler shaders)
- Check for overheating
- Increase TDR timeout (registry edit)

### 8.4 Swapchain Out of Date

**Symptoms**:
- Error: `Outdated` on present
- Resize causes black screen

**Resolutions**:
- Ensure window resize events are handled
- Surface recreation happens automatically in Bevy 0.13.2
- Check for minimized window (0x0 dimensions)

---

## Appendix A: WGPU 0.19 API Quick Reference

### Key Types

```rust
// Adapter selection
wgpu::Backends::VULKAN | wgpu::Backends::DX12
wgpu::PowerPreference::HighPerformance

// Surface configuration
wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: wgpu::TextureFormat::Bgra8UnormSrgb,
    width: 1920,
    height: 1080,
    present_mode: wgpu::PresentMode::Fifo,
    alpha_mode: wgpu::CompositeAlphaMode::Auto,
    view_formats: vec![],
}

// Present modes
wgpu::PresentMode::Fifo         // VSync on
wgpu::PresentMode::Immediate    // VSync off
wgpu::PresentMode::Mailbox      // Low-latency vsync
```

### Feature Flags Reference

| Feature | Bevy 0.13.2 Default | Notes |
|---------|---------------------|-------|
| `TEXTURE_BINDING_ARRAY` | No | Enable for texture arrays |
| `PUSH_CONSTANTS` | No | 256 bytes max |
| `SAMPLER_ANISOTROPY` | No | Requires feature enable |
| `MAPPABLE_PRIMARY_BUFFERS` | No | Faster CPU-GPU transfers |

---

*Document Version: 1.0*  
*Compatible with: Bevy 0.13.2, WGPU 0.19*  
*Last Updated: Rose Online Client Diagnostics Suite*

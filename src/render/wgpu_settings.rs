//! wgpu instance configuration: backend choice and debug/validation flags.

use bevy::render::settings::{Backends, InstanceFlags, PowerPreference, WgpuFeatures, WgpuSettings};
use wgpu::{Backend, DeviceType};

/// Builds the renderer's [`WgpuSettings`] with wgpu's debug/validation instance
/// flags turned off.
///
/// wgpu defaults to `DEBUG | VALIDATION | VALIDATION_INDIRECT_CALL` in debug builds,
/// and Bevy keeps `VALIDATION_INDIRECT_CALL` even in release while DX12 is an
/// allowed backend. That flag runs a validation compute pass and per-draw CPU work
/// for every indirect draw, and Bevy's GPU-driven renderer draws almost everything
/// indirectly.
///
/// Dropping `VALIDATION_INDIRECT_CALL` is only safe when the backend is not DX12:
/// on DX12 wgpu relies on that pass so `instance_index`/`vertex_index` include the
/// indirect `first_instance`/`base_vertex` (see `InstanceFlags::VALIDATION_INDIRECT_CALL`;
/// Bevy's release config keeps it for DX12 for the same reason). So the adapters are
/// probed once: if wgpu would select a non-DX12 hardware adapter anyway, the backend is
/// pinned to it and the flag is dropped. Otherwise the flag stays and only
/// `DEBUG`/`VALIDATION` are dropped.
///
/// Any flag can still be turned back on per session through wgpu's environment
/// variables: `WGPU_VALIDATION=1`, `WGPU_DEBUG=1`, `WGPU_VALIDATION_INDIRECT_CALL=1`.
pub fn create_wgpu_settings() -> WgpuSettings {
    let defaults = WgpuSettings::default();

    let (backends, instance_flags) = match probe_selected_backend(defaults.power_preference) {
        Some(backend) if backend != Backend::Dx12 => {
            log::info!(
                "[WGPU] {:?} adapter selected: pinning backend, debug/validation flags off",
                backend
            );
            (Backends::from(backend), InstanceFlags::empty())
        }
        selected => {
            log::info!(
                "[WGPU] Backend {:?}: keeping VALIDATION_INDIRECT_CALL (required on DX12), debug/validation flags off",
                selected
            );
            (Backends::all(), InstanceFlags::VALIDATION_INDIRECT_CALL)
        }
    };

    WgpuSettings {
        backends: Some(backends),
        instance_flags: instance_flags.with_env(),
        // Keep problematic bindless features disabled for stability,
        // but allow texture binding arrays needed by TerrainMaterial.
        disabled_features: Some(
            WgpuFeatures::BUFFER_BINDING_ARRAY
                | WgpuFeatures::STORAGE_RESOURCE_BINDING_ARRAY
                | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY,
        ),
        ..defaults
    }
}

/// Returns the backend of the adapter Bevy would select from `Backends::all()`,
/// when that is a hardware (discrete/integrated) GPU and the choice is predictable.
///
/// Mirrors wgpu-core's `request_adapter`: adapters are gathered in backend order
/// (Vulkan, Metal, DX12, GL), then stable-sorted by device type for the power
/// preference, and the first one wins. GL is not probed (creating a GL context is
/// slow); GL adapters report `DeviceType::Other`, which ranks below any hardware
/// adapter, so they cannot beat a hardware winner. Surface compatibility is not
/// checked (no window yet); hardware Vulkan/DX12/Metal adapters support the platform
/// surface.
fn probe_selected_backend(power_preference: PowerPreference) -> Option<Backend> {
    // Bevy selects by name / software fallback when these are set; not modelled here.
    if std::env::var_os("WGPU_ADAPTER_NAME").is_some()
        || std::env::var_os("WGPU_FORCE_FALLBACK_ADAPTER").is_some()
    {
        return None;
    }

    let probe_backends = Backends::VULKAN | Backends::METAL | Backends::DX12;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: probe_backends,
        flags: InstanceFlags::empty(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let mut adapters: Vec<_> = bevy::tasks::block_on(instance.enumerate_adapters(probe_backends))
        .iter()
        .map(|adapter| adapter.get_info())
        .collect();

    let prefer_integrated_gpu = match power_preference {
        PowerPreference::LowPower => Some(true),
        PowerPreference::HighPerformance => Some(false),
        PowerPreference::None => None,
    };
    if let Some(prefer_integrated_gpu) = prefer_integrated_gpu {
        // Same ranking as wgpu-core `request_adapter` (stable sort).
        adapters.sort_by_key(|info| match info.device_type {
            DeviceType::DiscreteGpu if prefer_integrated_gpu => 2,
            DeviceType::IntegratedGpu if prefer_integrated_gpu => 1,
            DeviceType::DiscreteGpu => 1,
            DeviceType::IntegratedGpu => 2,
            DeviceType::Other => 3,
            DeviceType::VirtualGpu => 4,
            DeviceType::Cpu => 5,
        });
    }

    let selected = adapters.first()?;
    log::info!(
        "[WGPU] Adapter probe: {} ({:?}, {:?})",
        selected.name,
        selected.backend,
        selected.device_type
    );
    matches!(
        selected.device_type,
        DeviceType::DiscreteGpu | DeviceType::IntegratedGpu
    )
    .then_some(selected.backend)
}

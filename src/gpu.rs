use ash::{vk, Entry};
use std::ffi::{CStr, CString};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuKind {
    Discrete,
    Integrated,
    Virtual,
    Cpu,
    Other,
}

impl GpuKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Discrete => "Dedicated",
            Self::Integrated => "Integrated",
            Self::Virtual => "Virtual",
            Self::Cpu => "Software",
            Self::Other => "Other",
        }
    }

    pub fn is_discrete(&self) -> bool {
        matches!(self, Self::Discrete)
    }
}

#[derive(Clone, Debug)]
pub struct GpuInfo {
    pub name: String,
    pub kind: GpuKind,
    pub vendor_id: u32,
    pub device_id: u32,
    pub api_version: u32,
    pub device_local_memory: u64,
    pub bc_compression: bool,
    pub graphics_queue: bool,
    pub max_texture_size: u32,
}

impl GpuInfo {
    pub fn compatible(&self) -> bool {
        self.graphics_queue
            && self.api_version >= vk::API_VERSION_1_1
            && self.max_texture_size >= 8192
            && !matches!(self.kind, GpuKind::Cpu)
    }

    pub fn score(&self) -> u64 {
        let kind = match self.kind {
            // A small dedicated adapter can be faster, but its hard VRAM ceiling is
            // also the most common cause of WebGPU device loss. Prefer an integrated
            // adapter with shared memory when the discrete adapter has under 4 GiB.
            GpuKind::Discrete if self.device_local_memory < 4 * 1024 * 1024 * 1024 => 700,
            GpuKind::Discrete => 1_000,
            GpuKind::Integrated => 800,
            GpuKind::Virtual => 300,
            GpuKind::Other => 200,
            GpuKind::Cpu => 0,
        };
        let memory_gib = (self.device_local_memory / (1024 * 1024 * 1024)).min(8);
        kind + memory_gib * 25 + u64::from(self.bc_compression) * 100
    }

    pub fn api_label(&self) -> String {
        format!(
            "{}.{}.{}",
            vk::api_version_major(self.api_version),
            vk::api_version_minor(self.api_version),
            vk::api_version_patch(self.api_version)
        )
    }

    pub fn memory_label(&self) -> String {
        if self.device_local_memory == 0 {
            "shared memory".to_owned()
        } else {
            format!(
                "{:.1} GiB",
                self.device_local_memory as f64 / (1024.0 * 1024.0 * 1024.0)
            )
        }
    }

    pub fn display_label(&self) -> String {
        format!(
            "{} · {} · {}",
            self.name,
            self.kind.label(),
            self.memory_label()
        )
    }

    pub fn readiness(&self) -> String {
        if matches!(self.kind, GpuKind::Cpu) {
            return "software rendering is not accepted".to_owned();
        }
        if !self.graphics_queue {
            return "missing a graphics queue".to_owned();
        }
        if self.api_version < vk::API_VERSION_1_1 {
            return format!("Vulkan {} is too old", self.api_label());
        }
        if self.max_texture_size < 8192 {
            return "maximum texture size is below 8192".to_owned();
        }
        if !self.bc_compression {
            return format!("ready with texture fallback · Vulkan {}", self.api_label());
        }
        format!("ready · Vulkan {} · BC textures", self.api_label())
    }

    pub fn stability_note(&self) -> &'static str {
        if self.kind.is_discrete() && self.device_local_memory < 4 * 1024 * 1024 * 1024 {
            "limited dedicated memory; use medium game textures"
        } else if matches!(self.kind, GpuKind::Integrated) {
            "shared-memory stability fallback"
        } else {
            "recommended for high-performance WebGPU"
        }
    }
}

pub fn probe() -> Result<Vec<GpuInfo>, String> {
    let entry = unsafe { Entry::load() }
        .map_err(|error| format!("Vulkan loader is unavailable: {error}"))?;
    let application_name = CString::new("fiveplayonlinux").expect("static application name");
    let application_info = vk::ApplicationInfo::default()
        .application_name(&application_name)
        .application_version(vk::make_api_version(0, 0, 2, 0))
        .engine_name(&application_name)
        .engine_version(vk::make_api_version(0, 0, 2, 0))
        .api_version(vk::API_VERSION_1_1);
    let create_info = vk::InstanceCreateInfo::default().application_info(&application_info);
    let instance = unsafe { entry.create_instance(&create_info, None) }
        .map_err(|error| format!("Vulkan initialization failed: {error}"))?;

    let result = (|| {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .map_err(|error| format!("GPU enumeration failed: {error}"))?;
        let mut devices = Vec::with_capacity(physical_devices.len());
        for device in physical_devices {
            let properties = unsafe { instance.get_physical_device_properties(device) };
            let features = unsafe { instance.get_physical_device_features(device) };
            let memory = unsafe { instance.get_physical_device_memory_properties(device) };
            let queues = unsafe { instance.get_physical_device_queue_family_properties(device) };
            let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            let kind = match properties.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => GpuKind::Discrete,
                vk::PhysicalDeviceType::INTEGRATED_GPU => GpuKind::Integrated,
                vk::PhysicalDeviceType::VIRTUAL_GPU => GpuKind::Virtual,
                vk::PhysicalDeviceType::CPU => GpuKind::Cpu,
                _ => GpuKind::Other,
            };
            let device_local_memory = memory.memory_heaps[..memory.memory_heap_count as usize]
                .iter()
                .filter(|heap| heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
                .map(|heap| heap.size)
                .sum();
            let graphics_queue = queues.iter().any(|queue| {
                queue.queue_count > 0 && queue.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            });
            devices.push(GpuInfo {
                name,
                kind,
                vendor_id: properties.vendor_id,
                device_id: properties.device_id,
                api_version: properties.api_version,
                device_local_memory,
                bc_compression: features.texture_compression_bc == vk::TRUE,
                graphics_queue,
                max_texture_size: properties.limits.max_image_dimension2_d,
            });
        }
        Ok(devices)
    })();

    unsafe { instance.destroy_instance(None) };
    result
}

pub fn compatible_devices(devices: &[GpuInfo]) -> Vec<GpuInfo> {
    let mut compatible: Vec<_> = devices
        .iter()
        .filter(|device| device.compatible())
        .cloned()
        .collect();
    compatible.sort_by_key(|device| std::cmp::Reverse(device.score()));
    compatible
}

pub fn select(
    devices: &[GpuInfo],
    requested_name: Option<&str>,
    integrated: bool,
) -> Option<GpuInfo> {
    let compatible = compatible_devices(devices);
    if let Some(name) = requested_name {
        if let Some(device) = compatible.iter().find(|device| device.name == name) {
            return Some(device.clone());
        }
    }
    if integrated {
        compatible
            .iter()
            .find(|device| matches!(device.kind, GpuKind::Integrated))
            .cloned()
            .or_else(|| compatible.first().cloned())
    } else {
        compatible.first().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, kind: GpuKind, memory_gib: u64) -> GpuInfo {
        GpuInfo {
            name: name.to_owned(),
            kind,
            vendor_id: 0,
            device_id: 0,
            api_version: vk::API_VERSION_1_3,
            device_local_memory: memory_gib * 1024 * 1024 * 1024,
            bc_compression: true,
            graphics_queue: true,
            max_texture_size: 16384,
        }
    }

    #[test]
    fn integrated_device_is_safer_than_low_memory_discrete() {
        let devices = vec![
            device("integrated", GpuKind::Integrated, 16),
            device("dedicated", GpuKind::Discrete, 2),
        ];
        assert_eq!(select(&devices, None, false).unwrap().name, "integrated");
    }

    #[test]
    fn capable_dedicated_device_is_recommended() {
        let devices = vec![
            device("integrated", GpuKind::Integrated, 16),
            device("dedicated", GpuKind::Discrete, 8),
        ];
        assert_eq!(select(&devices, None, false).unwrap().name, "dedicated");
    }

    #[test]
    fn cpu_adapter_is_not_compatible() {
        let software = device("software", GpuKind::Cpu, 8);
        assert!(!software.compatible());
        assert!(software.readiness().contains("software rendering"));
    }
}

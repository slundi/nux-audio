/// Runtime device registry loaded from the embedded `devices_registry.yaml`.
///
/// `devices_registry.yaml` is embedded at compile time via `include_str!` so
/// there is no file-system dependency at runtime.  Edit it to add or modify
/// supported devices; the change takes effect on the next build.
use serde::Deserialize;
use std::sync::LazyLock;

/// Specification for a single Nux device, as defined in `devices_registry.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceSpec {
    /// Internal slug (snake_case, unique across all devices).
    pub key: String,
    /// Human-readable product name.
    pub name: String,
    /// Short model code used in SysEx headers and CLI output (e.g. `"MG-30"`).
    pub model_code: String,
    /// SysEx device byte — matches `device_id` in `devices.yaml`.
    pub device_id: u8,
    /// MIDI channel (1-based).
    pub midi_channel: u8,
    /// Number of user preset slots on this device.
    pub preset_count: u8,
    /// Prefix of the OS MIDI port name for this device, used for auto-detection.
    pub midi_port_prefix: String,
    /// Effect slot names in signal-chain order (e.g. `["wah", "comp", "amp", …]`).
    pub signal_chain: Vec<String>,
}

#[derive(Deserialize)]
struct DevicesFile {
    device: Vec<DeviceSpec>,
}

/// The runtime device registry.
pub struct DeviceRegistry {
    devices: Vec<DeviceSpec>,
}

impl DeviceRegistry {
    fn load() -> Self {
        let raw = include_str!("../devices_registry.yaml");
        let file: DevicesFile = serde_saphyr::from_str(raw)
            .expect("devices_registry.yaml is embedded at compile time and must be valid YAML");
        Self {
            devices: file.device,
        }
    }

    /// Find the device whose `midi_port_prefix` is a prefix of `port_name`.
    ///
    /// Longest match wins — ensures `"NUX Mighty Plug Pro"` matches the Pro
    /// spec rather than the basic `"NUX Mighty Plug"` spec.
    pub fn match_port(&self, port_name: &str) -> Option<&DeviceSpec> {
        self.devices
            .iter()
            .filter(|d| port_name.starts_with(d.midi_port_prefix.as_str()))
            .max_by_key(|d| d.midi_port_prefix.len())
    }

    /// Look up a device by its SysEx device ID byte.
    pub fn by_device_id(&self, id: u8) -> Option<&DeviceSpec> {
        self.devices.iter().find(|d| d.device_id == id)
    }

    /// Look up a device by its model code (e.g. `"MG-30"`).
    pub fn by_model_code(&self, code: &str) -> Option<&DeviceSpec> {
        self.devices.iter().find(|d| d.model_code == code)
    }

    /// All registered device specifications.
    pub fn all(&self) -> &[DeviceSpec] {
        &self.devices
    }
}

/// Global device registry, populated from `devices_registry.yaml` on first access.
pub static REGISTRY: LazyLock<DeviceRegistry> = LazyLock::new(DeviceRegistry::load);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_loads_and_is_non_empty() {
        assert!(!REGISTRY.all().is_empty());
    }

    #[test]
    fn test_match_port_mg30() {
        let spec = REGISTRY.match_port("NUX MG-30").unwrap();
        assert_eq!(spec.model_code, "MG-30");
        assert_eq!(spec.device_id, 0x09);
        assert_eq!(spec.preset_count, 128);
    }

    #[test]
    fn test_match_port_with_os_suffix() {
        // OS may append port suffixes like " MIDI 1"
        let spec = REGISTRY.match_port("NUX MG-30 MIDI 1").unwrap();
        assert_eq!(spec.model_code, "MG-30");
    }

    #[test]
    fn test_match_port_longest_prefix_wins() {
        // "Mighty Plug Pro" must not match the basic "Mighty Plug" spec
        let spec = REGISTRY.match_port("NUX Mighty Plug Pro").unwrap();
        assert_eq!(spec.model_code, "MP-3");
        assert_eq!(spec.device_id, 0x03);
    }

    #[test]
    fn test_match_port_basic_plug() {
        let spec = REGISTRY.match_port("NUX Mighty Plug").unwrap();
        assert_eq!(spec.model_code, "MP-2");
        assert_eq!(spec.device_id, 0x01);
    }

    #[test]
    fn test_match_port_unknown() {
        assert!(REGISTRY.match_port("Roland TD-17").is_none());
        assert!(REGISTRY.match_port("").is_none());
    }

    #[test]
    fn test_by_device_id() {
        assert_eq!(
            REGISTRY.by_device_id(0x09).map(|d| d.model_code.as_str()),
            Some("MG-30")
        );
        assert!(REGISTRY.by_device_id(0xFF).is_none());
    }

    #[test]
    fn test_by_model_code() {
        assert_eq!(
            REGISTRY.by_model_code("MP-3").map(|d| d.device_id),
            Some(0x03)
        );
        assert!(REGISTRY.by_model_code("UNKNOWN").is_none());
    }

    #[test]
    fn test_signal_chain_mg30() {
        let spec = REGISTRY.match_port("NUX MG-30").unwrap();
        assert_eq!(spec.signal_chain.len(), 11);
        assert_eq!(spec.signal_chain[0], "wah");
        assert_eq!(spec.signal_chain[10], "vol");
    }

    #[test]
    fn test_signal_chain_mighty_basic() {
        let spec = REGISTRY.match_port("NUX Mighty Plug").unwrap();
        assert_eq!(spec.signal_chain.len(), 8);
        assert!(!spec.signal_chain.contains(&"wah".to_string()));
        assert!(!spec.signal_chain.contains(&"efx".to_string()));
    }

    #[test]
    fn test_all_device_ids_unique() {
        let ids: Vec<u8> = REGISTRY.all().iter().map(|d| d.device_id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            ids.len(),
            sorted.len(),
            "duplicate device_id found in devices_registry.yaml"
        );
    }

    #[test]
    fn test_all_model_codes_unique() {
        let codes: Vec<&str> = REGISTRY
            .all()
            .iter()
            .map(|d| d.model_code.as_str())
            .collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            codes.len(),
            sorted.len(),
            "duplicate model_code found in devices_registry.yaml"
        );
    }
}

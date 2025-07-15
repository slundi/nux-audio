use crate::preset::Preset;
use anyhow::{anyhow, Result};
use log::error;
use midir::{MidiInput, MidiOutput};
use std::collections::HashMap;

/// Manufacturer list.
///
/// When implementing other device, same manufacturer will probably share some attributes and methods.
#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum Manufacturer {
    Nux,
}

#[derive(Debug)]
pub struct Device {
    pub manufacturer: Manufacturer,
    pub model: String,
    /// The inferred name of the MIDI device.
    pub name: String,
    /// A map of input port names to their `midir` indices.
    pub inputs: HashMap<String, usize>,
    /// A map of output port names to their `midir` indices.
    pub outputs: HashMap<String, usize>,
}

impl Device {
    /// Attempts to extract a more generic device name from the full MIDI port name.
    ///
    /// This function is heuristic and heavily relies on OS/driver naming conventions.
    /// Examples of conventions:
    /// - "Device Name MIDI 1" -> "Device Name"
    /// - "Device Name (Port 1)" -> "Device Name"
    /// - "Device Name Port 1" -> "Device Name" (if 'Port' isn't part of the actual device name)
    /// - "Device Name" -> "Device Name"
    ///
    /// For a more robust solution, lower-level libraries (like `rusb` for USB)
    /// or OS-specific APIs would be required.
    fn extract_device_name(port_name: &str) -> String {
        let mut name = port_name.to_string();

        // Remove common suffixes indicating a specific port or instance number
        name = name.replace(" MIDI 1", "");
        name = name.replace(" MIDI 2", "");
        name = name.replace(" (Port 1)", "");
        name = name.replace(" (Port 2)", "");
        name = name.replace(" Port 1", "");
        name = name.replace(" Port 2", "");
        name = name.replace(" MIDI", ""); // Remove " MIDI" if at the end

        // Attempt to remove instance numbers or grouping parentheses (often on Windows)
        // E.g., "Focusrite USB MIDI (ABCDE12345)" -> "Focusrite USB MIDI"
        if let Some(pos) = name.rfind(" (") {
            let potential_suffix = &name[pos + 1..]; // Start from '(' to include it in the suffix
            if let Some(stripped_suffix) = potential_suffix.strip_suffix(')') {
                // Check if it looks like an instance ID (e.g., contains numbers/hex chars)
                let inner = stripped_suffix.trim_start_matches('('); // Remove leading '('
                if inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    name = name[..pos].to_string();
                }
            }
        }

        // Final trim for leading/trailing spaces
        name.trim().to_string()
    }

    /// Extract supported device from the MIDI device name
    fn extract_supported(value: &str) -> Option<(Manufacturer, String)> {
        if value.starts_with("NUX MG-30") {
            return Some((Manufacturer::Nux, "MG-30".to_string()));
        }
        None
    }

    /// Detect supported device(s) and return a list of available device with their MIDI ports.
    pub fn detect() -> Vec<Self> {
        let midi_in =
            MidiInput::new("MIDI Input Lister").expect("Cannot create a MIDI input lister");
        let midi_out =
            MidiOutput::new("MIDI Output Lister").expect("Cannot create a MIDI output lister");

        let mut devices: HashMap<String, Self> = HashMap::new();

        // inputs
        for (i, p) in midi_in.ports().iter().enumerate() {
            match midi_in.port_name(p) {
                Ok(port_name) => {
                    // println!("{}: {}", i, port_name);

                    // Attempt to extract a "generic" device name from the port name.
                    // This is the trickiest part and depends on system naming conventions.
                    let device_name = Self::extract_device_name(&port_name);
                    if let Some((manufacturer, model)) = Self::extract_supported(&device_name) {
                        let device_entry =
                            devices.entry(device_name.clone()).or_insert_with(|| Self {
                                manufacturer,
                                model,
                                name: device_name,
                                inputs: HashMap::new(),
                                outputs: HashMap::new(),
                            });
                        device_entry.inputs.insert(port_name, i);
                    }
                }
                Err(e) => error!("Cannot get input port: {e}"),
            }
        }

        // outputs
        for (i, p) in midi_out.ports().iter().enumerate() {
            match midi_out.port_name(p) {
                Ok(port_name) => {
                    // println!("{}: {}", i, port_name);

                    let device_name = Self::extract_device_name(&port_name);

                    if let Some((manufacturer, model)) = Self::extract_supported(&device_name) {
                        let device_entry =
                            devices.entry(device_name.clone()).or_insert_with(|| Self {
                                manufacturer,
                                model,
                                name: device_name.to_owned(),
                                inputs: HashMap::new(),
                                outputs: HashMap::new(),
                            });
                        device_entry.outputs.insert(port_name, i);
                    }
                }
                Err(e) => error!("Cannot get output port: {e}"),
            }
        }

        devices.into_values().collect()
    }
}

pub struct ControllerState<'a> {
    pub device: Device,
    pub device_connected: bool,
    pub device_version: (u8, u8, u8),
    // current_preset_data,
    // selected_effect_option,
    // midi_output: Option<MidiOutput>,
    // midi_input: Option<MidiInput>,
    pub active: bool,
    pub presets: Vec<Preset<'a>>,
    pub is_fetching_preset: bool,
}

/// Extract version from message. SysEx bytes must have been removed.
pub fn extract_version(response: &[u8]) -> Result<String> {
    if response.len() < 6 {
        return Err(anyhow!("Unexpected SysEx response length: {:?}", response));
    }
    let version_bytes = &response[4..response.len() - 1];
    let version = String::from_utf8(version_bytes.to_vec())?;
    let version = version.trim_end_matches('\0');
    let version = version.trim().to_string();

    if let Some(value) = version.split_once("MG-30") {
        return Ok(crate::utils::clean_string(value.0));
    }
    Err(anyhow!("Cannot get version from value {version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test for basic names that don't need any cleaning
    #[test]
    fn test_extract_device_name_basic() {
        assert_eq!(
            super::Device::extract_device_name("My Awesome Synth"),
            "My Awesome Synth"
        );
        assert_eq!(
            super::Device::extract_device_name("Another MIDI Device"),
            "Another Device"
        );
        assert_eq!(
            super::Device::extract_device_name("Roland TD-17"),
            "Roland TD-17"
        );
    }

    // Test cases with " MIDI X" suffix
    #[test]
    fn test_extract_device_name_midi_suffix() {
        assert_eq!(
            super::Device::extract_device_name("Device Name MIDI 1"),
            "Device Name"
        );
        assert_eq!(
            super::Device::extract_device_name("Synth Pro MIDI 2"),
            "Synth Pro"
        );
        assert_eq!(
            super::Device::extract_device_name("USB Interface MIDI"),
            "USB Interface"
        );
        // Ensure it doesn't remove "MIDI" if it's part of the core name
        assert_eq!(
            super::Device::extract_device_name("Awesome MIDI Controller"),
            "Awesome Controller"
        );
    }

    // Test cases with " (Port X)" suffix
    #[test]
    fn test_extract_device_name_port_suffix_parentheses() {
        assert_eq!(
            super::Device::extract_device_name("Interface (Port 1)"),
            "Interface"
        );
        assert_eq!(
            super::Device::extract_device_name("Sound Card (Port 2)"),
            "Sound Card"
        );
        assert_eq!(
            super::Device::extract_device_name("My Synth (Port 3)"),
            "My Synth (Port 3)"
        ); // Should not remove if not "Port 1" or "Port 2"
    }

    // Test cases with " Port X" suffix (no parentheses)
    #[test]
    fn test_extract_device_name_port_suffix_no_parentheses() {
        assert_eq!(
            super::Device::extract_device_name("Focusrite USB MIDI Port 1"),
            "Focusrite USB"
        );
        assert_eq!(
            super::Device::extract_device_name("UMC404HD Port 2"),
            "UMC404HD"
        );
        assert_eq!(
            super::Device::extract_device_name("Another Device Port 3"),
            "Another Device Port 3"
        ); // Should not remove if not "Port 1" or "Port 2"
    }

    // Test cases with instance IDs in parentheses (common on Windows)
    #[test]
    fn test_extract_device_name_instance_id_suffix() {
        assert_eq!(
            super::Device::extract_device_name("Device (ABC1234)"),
            "Device"
        );
        assert_eq!(
            super::Device::extract_device_name("MIDI Interface (DEF5678)"),
            "MIDI Interface"
        );
        assert_eq!(
            super::Device::extract_device_name("Awesome Thing (0000-0000-0000)"),
            "Awesome Thing"
        );
        // Should not remove if it doesn't look like an instance ID
        assert_eq!(
            super::Device::extract_device_name("Guitar (Input)"),
            "Guitar"
        );
        assert_eq!(super::Device::extract_device_name("Mixer (Main)"), "Mixer");
    }

    // Test cases with leading/trailing spaces
    #[test]
    fn test_extract_device_name_trimming() {
        assert_eq!(
            super::Device::extract_device_name("  My Device  "),
            "My Device"
        );
        assert_eq!(
            super::Device::extract_device_name("Device Name MIDI 1   "),
            "Device Name"
        );
    }

    // Test cases combining multiple suffixes
    #[test]
    fn test_extract_device_name_combined_suffixes() {
        assert_eq!(
            super::Device::extract_device_name("Synth (ABC1234) MIDI 1"),
            "Synth"
        );
        assert_eq!(
            super::Device::extract_device_name("Interface MIDI 2 (DEF5678)"),
            "Interface"
        );
        assert_eq!(
            super::Device::extract_device_name("AudioBox USB (Port 1) MIDI 1"),
            "AudioBox USB"
        );
    }

    // Test with empty string (though midir usually gives names)
    #[test]
    fn test_extract_device_name_empty_string() {
        assert_eq!(super::Device::extract_device_name(""), "");
    }

    #[test]
    fn test_extract_supported_devices() {
        assert_eq!(Device::extract_supported("unsupported"), None);
        assert_eq!(Device::extract_supported("not NUX MG-30"), None);
        assert_eq!(Device::extract_supported("NUX MG30"), None);
        assert_eq!(
            Device::extract_supported("NUX MG-30"),
            Some((Manufacturer::Nux, String::from("MG-30")))
        );
        assert_eq!(
            Device::extract_supported("NUX MG-30 123ABC"),
            Some((Manufacturer::Nux, String::from("MG-30")))
        );
    }
}

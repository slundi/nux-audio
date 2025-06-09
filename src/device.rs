use anyhow::{anyhow, Result};

use crate::preset::Preset;

pub struct ControllerState<'a> {
    pub device_connected: bool,
    pub device_name: String,
    pub device_version: (u8, u8, u8),
    // current_preset_data,
    // selected_effect_option,
    // midi_output: Option<MidiOutput>,
    // midi_input: Option<MidiInput>,
    pub active: bool,
    pub presetes: Vec<Preset<'a>>,
    pub is_fetching_preset: bool,
}

pub fn extract_version(response: &[u8]) -> Result<String> {
    if response.len() < 6 {
        return Err(anyhow!("Unexpected SysEx response length: {:?}", response));
    }
    let version_bytes = &response[4..response.len() - 1];
    let version = String::from_utf8(version_bytes.to_vec())?.replace("\0", "");
    let version = version.trim().to_string();

    if let Some(value) = version.split_once("MG-30") {
        return Ok(crate::utils::clean_string(value.0));
    }
    Err(anyhow!("Cannot get version from value {version}"))
}

// function extractDeviceVersion(response: Uint8Array): Nux.DeviceVersion {
//   if (response.length < 6) {
//     console.warn("⚠️ Unexpected SysEx response length:", response);
//     return { version: "Unknown" };
//   }

//   const versionBytes = response.slice(4, response.length - 1); // Avoid reading out of bounds
//   let version = String.fromCharCode(...versionBytes).trim();

//   version = version.replace(/\0+$/, "").split("MG-30")[0].trim();

//   version = version.replace(/[^a-zA-Z0-9.\-]/g, "").trim();
//   console.log(`🎸 Device version: ${version}`);
//   return { version: version || "Unknown" };
// }

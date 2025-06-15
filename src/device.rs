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

pub fn version_to_tuple(version: &str) -> Result<(u8, u8, u8)> {
    if version.chars().filter(|&c| c == '.').count() != 2 {
        return Err(anyhow!("Invalid version format: expected X.Y.Z, got '{}'", version));
    }

    let mut parts = version.split('.');
    let major = parts.next().ok_or_else(|| anyhow!("Missing major version"))?.parse::<u8>()?;
    let minor = parts.next().ok_or_else(|| anyhow!("Missing minor version"))?.parse::<u8>()?;
    let patch = parts.next().ok_or_else(|| anyhow!("Missing patch version"))?.parse::<u8>()?;

    Ok((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version_to_tuple("0.0.0").unwrap(),(0, 0, 0));
        assert!(version_to_tuple("0.00").is_err());
        assert!(version_to_tuple("0.a.0").is_err());
        assert!(version_to_tuple("0.0.f0").is_err());
        assert!(version_to_tuple("192.168.1.1").is_err());
    }
}


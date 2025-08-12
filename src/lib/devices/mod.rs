pub mod mighty_air;
pub mod mighty_plug;
pub mod mighty_plug_pro;
pub mod mighty_space;

use mighty_air::MightyAir;
use mighty_plug::MightyPlug;
use mighty_plug_pro::MightyPlugPro;
use mighty_space::MightySpace;
use serde::{Deserialize, Serialize};
use std::fmt;

// NUX Device definitions and mappings
use crate::{
    poc::{Effect, MidiCommand, Preset},
    SYSEX_END, SYSEX_START,
};

pub const NUX_MANUFACTURER_ID: [u8; 3] = [0x00, 0x21, 0x21];

// Device identification constants
pub const MIGHTY_PLUG_DEVICE_ID: u8 = 0x01;
pub const MIGHTY_PLUG_PRO_DEVICE_ID: u8 = 0x02;
pub const MIGHTY_AIR_DEVICE_ID: u8 = 0x03;
pub const MIGHTY_SPACE_DEVICE_ID: u8 = 0x04;

// Base implementation for common NUX device functionality
#[derive(Debug, Clone)]
pub struct BaseNuxDevice {
    pub info: DeviceInfo,
    pub presets: Vec<Preset>,
    pub current_preset_id: Option<u8>,
    pub effects: Vec<Effect>,
}

impl BaseNuxDevice {
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            info,
            presets: Vec::new(),
            current_preset_id: None,
            effects: Vec::new(),
        }
    }

    fn build_sysex_message(&self, command_data: &[u8]) -> Vec<u8> {
        let mut message = vec![SYSEX_START];
        message.extend_from_slice(&NUX_MANUFACTURER_ID);
        message.push(self.info.device_id);
        message.extend_from_slice(command_data);
        message.push(SYSEX_END);
        message
    }

    fn validate_sysex(&self, data: &[u8]) -> bool {
        if data.len() < 6 {
            return false;
        }

        data[0] == SYSEX_START
            && data[1..4] == NUX_MANUFACTURER_ID
            && data[4] == self.info.device_id
            && data[data.len() - 1] == SYSEX_END
    }
}

impl NuxDevice for BaseNuxDevice {
    fn get_device_info(&self) -> &DeviceInfo {
        &self.info
    }

    fn get_presets(&self) -> &[Preset] {
        &self.presets
    }

    fn get_current_preset(&self) -> Option<&Preset> {
        self.current_preset_id
            .and_then(|id| self.presets.iter().find(|p| p.id == id))
    }

    fn set_preset(&mut self, preset_id: u8) -> Result<(), String> {
        if preset_id >= self.info.max_presets {
            return Err(format!(
                "Preset {} out of range (max: {})",
                preset_id,
                self.info.max_presets - 1
            ));
        }
        self.current_preset_id = Some(preset_id);
        Ok(())
    }

    fn get_effects(&self) -> &[Effect] {
        &self.effects
    }

    fn toggle_effect(&mut self, effect_id: u8) -> Result<(), String> {
        if let Some(effect) = self.effects.get_mut(effect_id as usize) {
            effect.enabled = !effect.enabled;
            Ok(())
        } else {
            Err(format!("Effect {} not found", effect_id))
        }
    }

    fn set_parameter(&mut self, effect_id: u8, param_name: &str, value: u8) -> Result<(), String> {
        if let Some(effect) = self.effects.get_mut(effect_id as usize) {
            if let Some(param) = effect.parameters.get_mut(param_name) {
                if value >= param.min_value && value <= param.max_value {
                    param.value = value;
                    Ok(())
                } else {
                    Err(format!(
                        "Value {} out of range for parameter {}",
                        value, param_name
                    ))
                }
            } else {
                Err(format!("Parameter {} not found", param_name))
            }
        } else {
            Err(format!("Effect {} not found", effect_id))
        }
    }

    fn generate_sysex(&self, command: &MidiCommand) -> Result<Vec<u8>, String> {
        let command_data = match command {
            MidiCommand::PresetChange(preset_id) => vec![0x01, *preset_id],
            MidiCommand::ParameterChange {
                effect_id,
                param_id,
                value,
            } => {
                vec![0x02, *effect_id, *param_id, *value]
            }
            MidiCommand::EffectToggle { effect_id, enabled } => {
                vec![0x03, *effect_id, if *enabled { 0x01 } else { 0x00 }]
            }
            MidiCommand::TunerOn => vec![0x04, 0x01],
            MidiCommand::TunerOff => vec![0x04, 0x00],
            MidiCommand::TapTempo => vec![0x05],
            MidiCommand::LooperRecord => vec![0x06, 0x01],
            MidiCommand::LooperPlay => vec![0x06, 0x02],
            MidiCommand::LooperStop => vec![0x06, 0x00],
            MidiCommand::DeviceInfo => vec![0x07],
            MidiCommand::Custom(data) => data.clone(),
        };

        Ok(self.build_sysex_message(&command_data))
    }

    fn parse_sysex(&mut self, data: &[u8]) -> Result<Option<MidiCommand>, String> {
        if !self.validate_sysex(data) {
            return Err("Invalid SysEx message format".to_string());
        }

        if data.len() < 7 {
            return Err("SysEx message too short".to_string());
        }

        let command_type = data[5];
        let command = match command_type {
            0x01 if data.len() >= 7 => Some(MidiCommand::PresetChange(data[6])),
            0x02 if data.len() >= 9 => Some(MidiCommand::ParameterChange {
                effect_id: data[6],
                param_id: data[7],
                value: data[8],
            }),
            0x03 if data.len() >= 8 => Some(MidiCommand::EffectToggle {
                effect_id: data[6],
                enabled: data[7] != 0,
            }),
            0x04 if data.len() >= 7 => {
                if data[6] == 0x01 {
                    Some(MidiCommand::TunerOn)
                } else {
                    Some(MidiCommand::TunerOff)
                }
            }
            0x05 => Some(MidiCommand::TapTempo),
            0x06 if data.len() >= 7 => match data[6] {
                0x01 => Some(MidiCommand::LooperRecord),
                0x02 => Some(MidiCommand::LooperPlay),
                0x00 => Some(MidiCommand::LooperStop),
                _ => None,
            },
            0x07 => Some(MidiCommand::DeviceInfo),
            _ => Some(MidiCommand::Custom(data[5..data.len() - 1].to_vec())),
        };

        Ok(command)
    }
}

/// Trait for NUX device capabilities
pub trait NuxDevice {
    fn get_device_info(&self) -> &DeviceInfo;
    fn get_presets(&self) -> &[Preset];
    fn get_current_preset(&self) -> Option<&Preset>;
    fn set_preset(&mut self, preset_id: u8) -> Result<(), String>;
    fn get_effects(&self) -> &[Effect];
    fn toggle_effect(&mut self, effect_id: u8) -> Result<(), String>;
    fn set_parameter(&mut self, effect_id: u8, param_name: &str, value: u8) -> Result<(), String>;
    fn generate_sysex(&self, command: &MidiCommand) -> Result<Vec<u8>, String>;
    fn parse_sysex(&mut self, data: &[u8]) -> Result<Option<MidiCommand>, String>;
}

/// NUX device types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceType {
    MightyPlug,
    MightyPlugPro,
    MightySpace,
    MightyAir,
    MightyLiteBt,
    Mighty8Bt,
    Mighty20Bt,
    Mighty40Bt,
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceModel {
    MightyPlug,
    MightyPlugPro,
    MightyAir,
    MightySpace,
    Mg30,
    // Mg101,
    // Mg300,
    // Mg400
    // MightyBass50bt
    // Ac 25/60/80
    // AmpAcademy?
    // MldBassPreamp?
    Unknown(String),
}

impl fmt::Display for DeviceModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceModel::MightyPlug => write!(f, "Mighty Plug"),
            DeviceModel::MightyPlugPro => write!(f, "Mighty Plug Pro"),
            DeviceModel::MightyAir => write!(f, "Mighty Air"),
            DeviceModel::MightySpace => write!(f, "Mighty Space"),
            DeviceModel::Mg30 => write!(f, "MG-30"),
            DeviceModel::Unknown(name) => write!(f, "Unknown({})", name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub model: DeviceModel,
    pub firmware_version: String,
    pub device_id: u8,
    pub max_presets: u8,
}

// Device detection utilities
pub struct DeviceDetector;

impl DeviceDetector {
    pub fn detect_from_midi_name(name: &str) -> Option<DeviceModel> {
        let name = name.to_uppercase();
        let name = name.as_str();
        if name.starts_with("NUX-NMP-03") {
            return Some(DeviceModel::MightyPlugPro);
        }
        if name.starts_with("NUX MG-30") {
            return Some(DeviceModel::Mg30);
        }
        None
    }

    /// Detect device from SysEx message
    pub fn detect_from_sysex(data: &[u8]) -> Option<DeviceModel> {
        if data.len() < 5 {
            return None;
        }

        // Check if it's a NUX SysEx message
        if data[0] != SYSEX_START || data[1..4] != NUX_MANUFACTURER_ID {
            return None;
        }

        let device_id = data[4];
        match device_id {
            MIGHTY_PLUG_DEVICE_ID => Some(DeviceModel::MightyPlug),
            MIGHTY_PLUG_PRO_DEVICE_ID => Some(DeviceModel::MightyPlugPro),
            MIGHTY_AIR_DEVICE_ID => Some(DeviceModel::MightyAir),
            MIGHTY_SPACE_DEVICE_ID => Some(DeviceModel::MightySpace),
            _ => Some(DeviceModel::Unknown(format!("Device ID: {}", device_id))),
        }
    }

    /// Create device instance based on detected model
    pub fn create_device(model: DeviceModel) -> Option<Box<dyn NuxDevice>> {
        match model {
            DeviceModel::MightyPlug => Some(Box::new(MightyPlug::default())),
            DeviceModel::MightyPlugPro => Some(Box::new(MightyPlugPro::default())),
            DeviceModel::MightyAir => Some(Box::new(MightyAir::default())),
            DeviceModel::MightySpace => Some(Box::new(MightySpace::default())),
            DeviceModel::Mg30 => todo!(),
            DeviceModel::Unknown(_) => None,
        }
    }
}

pub mod midi_utils {
    use super::*;

    /// Convert a 7-bit value to MIDI format (0-127)
    pub fn to_midi_value(value: f32, min: f32, max: f32) -> u8 {
        let normalized = (value - min) / (max - min);
        (normalized * 127.0).clamp(0.0, 127.0) as u8
    }

    /// Convert a MIDI value (0-127) to a float range
    pub fn from_midi_value(midi_value: u8, min: f32, max: f32) -> f32 {
        let normalized = midi_value as f32 / 127.0;
        min + normalized * (max - min)
    }

    /// Check if a MIDI message is a NUX SysEx message
    pub fn is_nux_sysex(data: &[u8]) -> bool {
        data.len() >= 5
            && data[0] == SYSEX_START
            && data[1..4] == NUX_MANUFACTURER_ID
            && data[data.len() - 1] == SYSEX_END
    }

    /// Extract device ID from a NUX SysEx message
    pub fn extract_device_id(data: &[u8]) -> Option<u8> {
        if is_nux_sysex(data) && data.len() >= 5 {
            Some(data[4])
        } else {
            None
        }
    }

    /// Create a simple MIDI CC (Control Change) message
    pub fn create_cc_message(channel: u8, controller: u8, value: u8) -> Vec<u8> {
        vec![0xB0 | (channel & 0x0F), controller & 0x7F, value & 0x7F]
    }

    /// Create a MIDI Program Change message
    pub fn create_pc_message(channel: u8, program: u8) -> Vec<u8> {
        vec![0xC0 | (channel & 0x0F), program & 0x7F]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let mighty_plug = MightyPlug::default();
        assert_eq!(mighty_plug.get_device_info().model, DeviceModel::MightyPlug);
        assert_eq!(mighty_plug.get_device_info().max_presets, 7);
    }

    #[test]
    fn test_sysex_generation() {
        let device = MightyPlug::default();
        let command = MidiCommand::PresetChange(3);
        let sysex = device.generate_sysex(&command).unwrap();

        assert_eq!(sysex[0], SYSEX_START);
        assert_eq!(sysex[1..4], NUX_MANUFACTURER_ID);
        assert_eq!(sysex[4], MIGHTY_PLUG_DEVICE_ID);
        assert_eq!(sysex[5], 0x01); // Preset change command
        assert_eq!(sysex[6], 3); // Preset ID
        assert_eq!(sysex[7], SYSEX_END);
    }

    #[test]
    fn test_device_detection() {
        let sysex = vec![
            SYSEX_START,
            NUX_MANUFACTURER_ID[0],
            NUX_MANUFACTURER_ID[1],
            NUX_MANUFACTURER_ID[2],
            MIGHTY_PLUG_PRO_DEVICE_ID,
            0x01,
            0x00,
            SYSEX_END,
        ];

        assert_eq!(
            DeviceDetector::detect_from_sysex(&sysex),
            Some(DeviceModel::MightyPlugPro)
        );
    }

    #[test]
    fn test_effect_parameter_modification() {
        let mut device = MightyPlug::default();

        // Test setting a parameter
        assert!(device.set_parameter(1, "gain", 100).is_ok()); // Amp gain

        // Test invalid effect ID
        assert!(device.set_parameter(99, "gain", 100).is_err());

        // Test invalid parameter name
        assert!(device.set_parameter(1, "invalid_param", 100).is_err());
    }

    #[test]
    fn test_midi_utils() {
        use midi_utils::*;

        assert_eq!(to_midi_value(0.5, 0.0, 1.0), 63);
        assert_eq!(to_midi_value(1.0, 0.0, 1.0), 127);

        assert!((from_midi_value(63, 0.0, 1.0) - 0.496).abs() < 0.01);

        let cc_msg = create_cc_message(0, 7, 100); // Volume CC on channel 1
        assert_eq!(cc_msg, vec![0xB0, 0x07, 0x64]);
    }
}

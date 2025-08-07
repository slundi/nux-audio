// NUX Device definitions and mappings
use crate::{DeviceType, EffectType, NuxDevice};
use std::collections::HashMap;

pub const NUX_MANUFACTURER_ID: [u8; 3] = [0x00, 0x21, 0x21];

// Device identification constants
pub const MIGHTY_PLUG_DEVICE_ID: u8 = 0x01;
pub const MIGHTY_PLUG_PRO_DEVICE_ID: u8 = 0x02;
pub const MIGHTY_AIR_DEVICE_ID: u8 = 0x03;
pub const MIGHTY_SPACE_DEVICE_ID: u8 = 0x04;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceModel {
    MightyPlug,
    MightyPlugPro,
    MightyAir,
    MightySpace,
    Mg30,
    // Mg101,
    // Mg30,
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
            DeviceModel::Unknown(name) => write!(f, "Unknown({})", name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub model: DeviceModel,
    pub firmware_version: String,
    pub device_id: u8,
    pub bluetooth_name: String,
    pub supports_dual_bluetooth: bool,
    pub max_presets: u8,
}

/// Mighty Plug device implementation
#[derive(Debug, Clone)]
pub struct MightyPlug {
    effect_mappings: HashMap<EffectType, u8>,
}

impl Default for MightyPlug {
    fn default() -> Self {
        let mut mappings = HashMap::new();
        // Effect ID mappings for Mighty Plug
        mappings.insert(EffectType::AmpClean, 0x00);
        mappings.insert(EffectType::AmpCrunch, 0x01);
        mappings.insert(EffectType::AmpLead, 0x02);
        mappings.insert(EffectType::Overdrive, 0x10);
        mappings.insert(EffectType::Distortion, 0x11);
        mappings.insert(EffectType::Chorus, 0x20);
        mappings.insert(EffectType::Flanger, 0x21);
        mappings.insert(EffectType::Phaser, 0x22);
        mappings.insert(EffectType::Delay, 0x30);
        mappings.insert(EffectType::Reverb, 0x31);
        mappings.insert(EffectType::Wah, 0x40);
        mappings.insert(EffectType::Compressor, 0x41);
        mappings.insert(EffectType::NoiseGate, 0x50);

        Self {
            effect_mappings: mappings,
        }
    }
}

impl NuxDevice for MightyPlug {
    fn device_type(&self) -> DeviceType {
        DeviceType::MightyPlug
    }

    fn supported_effects(&self) -> Vec<EffectType> {
        vec![
            EffectType::AmpClean,
            EffectType::AmpCrunch,
            EffectType::AmpLead,
            EffectType::Overdrive,
            EffectType::Distortion,
            EffectType::Chorus,
            EffectType::Flanger,
            EffectType::Phaser,
            EffectType::Delay,
            EffectType::Reverb,
            EffectType::Wah,
            EffectType::Compressor,
            EffectType::NoiseGate,
        ]
    }

    fn effect_id(&self, effect_type: EffectType) -> Option<u8> {
        self.effect_mappings.get(&effect_type).copied()
    }

    fn midi_channel(&self) -> u8 {
        0x00 // Channel 1
    }

    fn sysex_header(&self) -> Vec<u8> {
        vec![0xF0, 0x00, 0x20, 0x7E, 0x01] // NUX SysEx header for Mighty Plug
    }
}

/// Mighty Plug Pro device implementation
#[derive(Debug, Clone)]
pub struct MightyPlugPro {
    effect_mappings: HashMap<EffectType, u8>,
}

impl Default for MightyPlugPro {
    fn default() -> Self {
        let mut mappings = HashMap::new();
        // Extended effect mappings for Mighty Plug Pro
        mappings.insert(EffectType::AmpClean, 0x00);
        mappings.insert(EffectType::AmpCrunch, 0x01);
        mappings.insert(EffectType::AmpLead, 0x02);
        mappings.insert(EffectType::AmpBass, 0x03);
        mappings.insert(EffectType::AmpAcoustic, 0x04);
        mappings.insert(EffectType::Overdrive, 0x10);
        mappings.insert(EffectType::Distortion, 0x11);
        mappings.insert(EffectType::Fuzz, 0x12);
        mappings.insert(EffectType::TubeScreamer, 0x13);
        mappings.insert(EffectType::MetalZone, 0x14);
        mappings.insert(EffectType::Chorus, 0x20);
        mappings.insert(EffectType::Flanger, 0x21);
        mappings.insert(EffectType::Phaser, 0x22);
        mappings.insert(EffectType::Tremolo, 0x23);
        mappings.insert(EffectType::Vibrato, 0x24);
        mappings.insert(EffectType::Rotary, 0x25);
        mappings.insert(EffectType::Delay, 0x30);
        mappings.insert(EffectType::Echo, 0x31);
        mappings.insert(EffectType::Reverb, 0x32);
        mappings.insert(EffectType::Wah, 0x40);
        mappings.insert(EffectType::AutoWah, 0x41);
        mappings.insert(EffectType::Compressor, 0x42);
        mappings.insert(EffectType::NoiseGate, 0x50);
        mappings.insert(EffectType::Boost, 0x51);
        mappings.insert(EffectType::Tuner, 0x60);

        Self {
            effect_mappings: mappings,
        }
    }
}

impl NuxDevice for MightyPlugPro {
    fn device_type(&self) -> DeviceType {
        DeviceType::MightyPlugPro
    }

    fn supported_effects(&self) -> Vec<EffectType> {
        vec![
            EffectType::AmpClean,
            EffectType::AmpCrunch,
            EffectType::AmpLead,
            EffectType::AmpBass,
            EffectType::AmpAcoustic,
            EffectType::Overdrive,
            EffectType::Distortion,
            EffectType::Fuzz,
            EffectType::TubeScreamer,
            EffectType::MetalZone,
            EffectType::Chorus,
            EffectType::Flanger,
            EffectType::Phaser,
            EffectType::Tremolo,
            EffectType::Vibrato,
            EffectType::Rotary,
            EffectType::Delay,
            EffectType::Echo,
            EffectType::Reverb,
            EffectType::Wah,
            EffectType::AutoWah,
            EffectType::Compressor,
            EffectType::NoiseGate,
            EffectType::Boost,
            EffectType::Tuner,
        ]
    }

    fn effect_id(&self, effect_type: EffectType) -> Option<u8> {
        self.effect_mappings.get(&effect_type).copied()
    }

    fn midi_channel(&self) -> u8 {
        0x00 // Channel 1
    }

    fn sysex_header(&self) -> Vec<u8> {
        vec![0xF0, 0x00, 0x20, 0x7E, 0x02] // NUX SysEx header for Mighty Plug Pro
    }
}

/// Mighty 20BT device implementation
#[derive(Debug, Clone)]
pub struct Mighty20Bt {
    effect_mappings: HashMap<EffectType, u8>,
}

impl Default for Mighty20Bt {
    fn default() -> Self {
        let mut mappings = HashMap::new();
        // Effect mappings for Mighty 20BT
        mappings.insert(EffectType::AmpClean, 0x00);
        mappings.insert(EffectType::AmpCrunch, 0x01);
        mappings.insert(EffectType::AmpLead, 0x02);
        mappings.insert(EffectType::Overdrive, 0x10);
        mappings.insert(EffectType::Distortion, 0x11);
        mappings.insert(EffectType::Fuzz, 0x12);
        mappings.insert(EffectType::Chorus, 0x20);
        mappings.insert(EffectType::Flanger, 0x21);
        mappings.insert(EffectType::Delay, 0x30);
        mappings.insert(EffectType::Reverb, 0x31);
        mappings.insert(EffectType::Compressor, 0x40);
        mappings.insert(EffectType::NoiseGate, 0x50);

        Self {
            effect_mappings: mappings,
        }
    }
}

impl NuxDevice for Mighty20Bt {
    fn device_type(&self) -> DeviceType {
        DeviceType::Mighty20Bt
    }

    fn supported_effects(&self) -> Vec<EffectType> {
        vec![
            EffectType::AmpClean,
            EffectType::AmpCrunch,
            EffectType::AmpLead,
            EffectType::Overdrive,
            EffectType::Distortion,
            EffectType::Fuzz,
            EffectType::Chorus,
            EffectType::Flanger,
            EffectType::Delay,
            EffectType::Reverb,
            EffectType::Compressor,
            EffectType::NoiseGate,
        ]
    }

    fn effect_id(&self, effect_type: EffectType) -> Option<u8> {
        self.effect_mappings.get(&effect_type).copied()
    }

    fn midi_channel(&self) -> u8 {
        0x00 // Channel 1
    }

    fn sysex_header(&self) -> Vec<u8> {
        vec![0xF0, 0x00, 0x20, 0x7E, 0x03] // NUX SysEx header for Mighty 20BT
    }
}

/// Device factory for creating device instances
pub struct DeviceFactory;

impl DeviceFactory {
    pub fn create_device(device_type: DeviceType) -> Box<dyn NuxDevice> {
        match device_type {
            DeviceType::MightyPlug => Box::new(MightyPlug::default()),
            DeviceType::MightyPlugPro => Box::new(MightyPlugPro::default()),
            DeviceType::MightySpace => Box::new(MightyPlugPro::default()), // Same as Pro
            DeviceType::Mighty20Bt => Box::new(Mighty20Bt::default()),
            DeviceType::Mighty40Bt => Box::new(Mighty20Bt::default()), // Same as 20BT
            _ => Box::new(MightyPlug::default()),                      // Default fallback
        }
    }
}

// Device detection utilities
pub struct DeviceDetector;

impl DeviceDetector {
    /// Detect device from Bluetooth name
    pub fn detect_from_bluetooth_name(name: &str) -> Option<DeviceModel> {
        match name.to_uppercase().as_str() {
            "MIGHTY PLUG" => Some(DeviceModel::MightyPlug),
            "MIGHTY PLUG PRO" => Some(DeviceModel::MightyPlugPro),
            "MIGHTY AIR" => Some(DeviceModel::MightyAir),
            "MIGHTY SPACE" => Some(DeviceModel::MightySpace),
            _ if name.to_uppercase().contains("MIGHTY") => {
                Some(DeviceModel::Unknown(name.to_string()))
            }
            _ => None,
        }
    }

    pub fn detect_from_midi_name(name: &str) -> Option<DeviceModel> {
        let name = name.to_uppercase().as_str();
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
            DeviceModel::MightyPlug => Some(Box::new(MightyPlug::new())),
            DeviceModel::MightyPlugPro => Some(Box::new(MightyPlugPro::new())),
            DeviceModel::MightyAir => Some(Box::new(MightyAir::new())),
            DeviceModel::MightySpace => Some(Box::new(MightySpace::new())),
            DeviceModel::Unknown(_) => None,
        }
    }
}

// Device detection utilities
pub struct DeviceDetector;

impl DeviceDetector {
    /// Detect device from Bluetooth name
    pub fn detect_from_bluetooth_name(name: &str) -> Option<DeviceModel> {
        match name.to_uppercase().as_str() {
            "MIGHTY PLUG" => Some(DeviceModel::MightyPlug),
            "MIGHTY PLUG PRO" => Some(DeviceModel::MightyPlugPro),
            "MIGHTY AIR" => Some(DeviceModel::MightyAir),
            "MIGHTY SPACE" => Some(DeviceModel::MightySpace),
            _ if name.to_uppercase().contains("MIGHTY") => {
                Some(DeviceModel::Unknown(name.to_string()))
            }
            _ => None,
        }
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
            DeviceModel::MightyPlug => Some(Box::new(MightyPlug::new())),
            DeviceModel::MightyPlugPro => Some(Box::new(MightyPlugPro::new())),
            DeviceModel::MightyAir => Some(Box::new(MightyAir::new())),
            DeviceModel::MightySpace => Some(Box::new(MightySpace::new())),
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
        let mighty_plug = MightyPlug::new();
        assert_eq!(mighty_plug.get_device_info().model, DeviceModel::MightyPlug);
        assert_eq!(mighty_plug.get_device_info().max_presets, 7);
        assert!(mighty_plug.get_device_info().supports_dual_bluetooth);
    }

    #[test]
    fn test_sysex_generation() {
        let device = MightyPlug::new();
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
        assert_eq!(
            DeviceDetector::detect_from_bluetooth_name("MIGHTY PLUG"),
            Some(DeviceModel::MightyPlug)
        );

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
        let mut device = MightyPlug::new();

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

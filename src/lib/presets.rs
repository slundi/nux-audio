// Preset management and serialization
use crate::{DeviceType, EffectError, EffectType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Preset bank containing multiple presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetBank {
    pub name: String,
    pub device_type: DeviceType,
    pub presets: HashMap<u8, PresetData>, // Preset number -> Preset data
}

/// Serializable preset data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetData {
    pub name: String,
    pub device_type: DeviceType,
    pub effects: HashMap<String, EffectParameters>, // Effect type -> Parameters
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

/// Effect parameters for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectParameters {
    pub effect_type: String,
    pub enabled: bool,
    pub parameters: HashMap<String, u8>,
}

impl PresetBank {
    pub fn new(name: String, device_type: DeviceType) -> Self {
        Self {
            name,
            device_type,
            presets: HashMap::new(),
        }
    }

    pub fn add_preset(&mut self, preset_number: u8, preset: PresetData) {
        self.presets.insert(preset_number, preset);
    }

    pub fn get_preset(&self, preset_number: u8) -> Option<&PresetData> {
        self.presets.get(&preset_number)
    }

    pub fn remove_preset(&mut self, preset_number: u8) -> Option<PresetData> {
        self.presets.remove(&preset_number)
    }

    pub fn list_presets(&self) -> Vec<(u8, &str)> {
        let mut presets: Vec<_> = self
            .presets
            .iter()
            .map(|(num, preset)| (*num, preset.name.as_str()))
            .collect();
        presets.sort_by_key(|(num, _)| *num);
        presets
    }

    /// Save preset bank to JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load preset bank from JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(path)?;
        let bank = serde_json::from_str(&json)?;
        Ok(bank)
    }

    /// Convert entire preset bank to SysEx messages
    pub fn to_sysex(&self) -> Result<Vec<(u8, Vec<u8>)>, EffectError> {
        let mut sysex_messages = Vec::new();

        for (preset_num, preset_data) in &self.presets {
            let sysex = preset_data.to_sysex()?;
            sysex_messages.push(*preset_num, sysex);
        }

        Ok(sysex_messages)
    }
}

impl PresetData {
    pub fn new(name: String, device_type: DeviceType) -> Self {
        let now = chrono::Utc::now();
        Self {
            name,
            device_type,
            effects: HashMap::new(),
            created_at: now,
            modified_at: now,
        }
    }

    pub fn add_effect(&mut self, effect_type: EffectType, parameters: EffectParameters) {
        self.effects
            .insert(format!("{:?}", effect_type), parameters);
        self.modified_at = chrono::Utc::now();
    }

    pub fn remove_effect(&mut self, effect_type: EffectType) -> Option<EffectParameters> {
        self.modified_at = chrono::Utc::now();
        self.effects.remove(&format!("{:?}", effect_type))
    }

    pub fn get_effect(&self, effect_type: EffectType) -> Option<&EffectParameters> {
        self.effects.get(&format!("{:?}", effect_type))
    }

    /// Convert preset to SysEx message sequence
    pub fn to_sysex(&self) -> Result<Vec<u8>, EffectError> {
        use crate::DeviceFactory;
        let device = DeviceFactory::create_device(self.device_type);
        let mut sysex_data = Vec::new();

        // Start with preset header
        let mut header = device.sysex_header();
        header.extend(&[0x21]); // Load preset command
        sysex_data.extend(header);

        // Add each effect's parameters
        for (effect_name, effect_params) in &self.effects {
            let effect_type = match effect_name.as_str() {
                "AmpClean" => EffectType::AmpClean,
                "AmpCrunch" => EffectType::AmpCrunch,
                "AmpLead" => EffectType::AmpLead,
                "Overdrive" => EffectType::Overdrive,
                "Distortion" => EffectType::Distortion,
                "Chorus" => EffectType::Chorus,
                "Delay" => EffectType::Delay,
                "Reverb" => EffectType::Reverb,
                "Compressor" => EffectType::Compressor,
                _ => continue,
            };

            let effect_id = device
                .effect_id(effect_type)
                .ok_or(EffectError::UnsupportedEffect(effect_type))?;

            // Add effect data
            let mut effect_sysex = device.sysex_header();
            effect_sysex.extend(&[
                0x06, // Effect parameter change
                effect_id,
                if effect_params.enabled { 0x01 } else { 0x00 },
            ]);

            // Add parameters in order
            let param_order = [
                "drive", "tone", "level", "gain", "bass", "mid", "treble", "rate", "depth",
                "feedback", "mix", "time", "size", "damping",
            ];

            for param_name in &param_order {
                if let Some(value) = effect_params.parameters.get(*param_name) {
                    effect_sysex.push(*value);
                }
            }

            effect_sysex.push(0xF7); // End SysEx
            sysex_data.extend(effect_sysex);
        }

        sysex_data.push(0xF7); // End preset message
        Ok(sysex_data)
    }
}

impl EffectParameters {
    pub fn new(effect_type: EffectType) -> Self {
        Self {
            effect_type: format!("{:?}", effect_type),
            enabled: false,
            parameters: HashMap::new(),
        }
    }

    pub fn set_parameter(&mut self, name: &str, value: u8) {
        self.parameters.insert(name.to_string(), value);
    }

    pub fn get_parameter(&self, name: &str) -> Option<u8> {
        self.parameters.get(name).copied()
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

/// Preset factory for creating common presets
pub struct PresetFactory;

impl PresetFactory {
    /// Create a clean preset with basic amp settings
    pub fn create_clean_preset(device_type: DeviceType) -> PresetData {
        let mut preset = PresetData::new("Clean".to_string(), device_type);

        let mut amp_params = EffectParameters::new(EffectType::AmpClean);
        amp_params.enable();
        amp_params.set_parameter("gain", 20);
        amp_params.set_parameter("bass", 50);
        amp_params.set_parameter("mid", 50);
        amp_params.set_parameter("treble", 60);
        amp_params.set_parameter("presence", 40);
        amp_params.set_parameter("volume", 70);

        preset.add_effect(EffectType::AmpClean, amp_params);

        // Add subtle reverb
        let mut reverb_params = EffectParameters::new(EffectType::Reverb);
        reverb_params.enable();
        reverb_params.set_parameter("type", 0); // Room
        reverb_params.set_parameter("size", 30);
        reverb_params.set_parameter("mix", 20);

        preset.add_effect(EffectType::Reverb, reverb_params);

        preset
    }

    /// Create a rock preset with overdrive and delay
    pub fn create_rock_preset(device_type: DeviceType) -> PresetData {
        let mut preset = PresetData::new("Rock".to_string(), device_type);

        // Crunch amp
        let mut amp_params = EffectParameters::new(EffectType::AmpCrunch);
        amp_params.enable();
        amp_params.set_parameter("gain", 60);
        amp_params.set_parameter("bass", 60);
        amp_params.set_parameter("mid", 70);
        amp_params.set_parameter("treble", 65);
        amp_params.set_parameter("presence", 55);
        amp_params.set_parameter("volume", 75);

        preset.add_effect(EffectType::AmpCrunch, amp_params);

        // Overdrive boost
        let mut od_params = EffectParameters::new(EffectType::Overdrive);
        od_params.enable();
        od_params.set_parameter("drive", 40);
        od_params.set_parameter("tone", 60);
        od_params.set_parameter("level", 60);

        preset.add_effect(EffectType::Overdrive, od_params);

        // Delay
        let mut delay_params = EffectParameters::new(EffectType::Delay);
        delay_params.enable();
        delay_params.set_parameter("time", 35);
        delay_params.set_parameter("feedback", 25);
        delay_params.set_parameter("mix", 20);

        preset.add_effect(EffectType::Delay, delay_params);

        preset
    }

    /// Create a lead preset with high gain and effects
    pub fn create_lead_preset(device_type: DeviceType) -> PresetData {
        let mut preset = PresetData::new("Lead".to_string(), device_type);

        // Lead amp
        let mut amp_params = EffectParameters::new(EffectType::AmpLead);
        amp_params.enable();
        amp_params.set_parameter("gain", 80);
        amp_params.set_parameter("bass", 50);
        amp_params.set_parameter("mid", 75);
        amp_params.set_parameter("treble", 70);
        amp_params.set_parameter("presence", 60);
        amp_params.set_parameter("volume", 70);

        preset.add_effect(EffectType::AmpLead, amp_params);

        // Compressor for sustain
        let mut comp_params = EffectParameters::new(EffectType::Compressor);
        comp_params.enable();
        comp_params.set_parameter("threshold", 50);
        comp_params.set_parameter("ratio", 60);
        comp_params.set_parameter("makeup", 40);

        preset.add_effect(EffectType::Compressor, comp_params);

        // Delay for space
        let mut delay_params = EffectParameters::new(EffectType::Delay);
        delay_params.enable();
        delay_params.set_parameter("time", 30);
        delay_params.set_parameter("feedback", 30);
        delay_params.set_parameter("mix", 25);

        preset.add_effect(EffectType::Delay, delay_params);

        // Reverb for ambience
        let mut reverb_params = EffectParameters::new(EffectType::Reverb);
        reverb_params.enable();
        reverb_params.set_parameter("type", 1); // Hall
        reverb_params.set_parameter("size", 60);
        reverb_params.set_parameter("mix", 30);

        preset.add_effect(EffectType::Reverb, reverb_params);

        preset
    }
}

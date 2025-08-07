// Amp model effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone, PartialEq)]
pub enum AmpModel {
    Clean,
    Crunch,
    Lead,
    Bass,
    Acoustic,
    // Extended models for Pro devices
    Vintage,
    Modern,
    British,
    American,
    Metal,
}

impl AmpModel {
    fn to_effect_type(&self) -> EffectType {
        match self {
            AmpModel::Clean => EffectType::AmpClean,
            AmpModel::Crunch => EffectType::AmpCrunch,
            AmpModel::Lead => EffectType::AmpLead,
            AmpModel::Bass => EffectType::AmpBass,
            AmpModel::Acoustic => EffectType::AmpAcoustic,
            _ => EffectType::AmpClean, // Default fallback
        }
    }
}

#[derive(Debug, Clone)]
pub struct Amp {
    model: AmpModel,
    gain: u8,     // 0-100
    bass: u8,     // 0-100
    mid: u8,      // 0-100
    treble: u8,   // 0-100
    presence: u8, // 0-100
    volume: u8,   // 0-100
    enabled: bool,
}

impl Default for Amp {
    fn default() -> Self {
        Self {
            model: AmpModel::Clean,
            gain: 30,
            bass: 50,
            mid: 50,
            treble: 50,
            presence: 50,
            volume: 70,
            enabled: true, // Amp is usually always enabled
        }
    }
}

impl Amp {
    pub fn new(
        model: AmpModel,
        gain: u8,
        bass: u8,
        mid: u8,
        treble: u8,
        presence: u8,
        volume: u8,
    ) -> Self {
        Self {
            model,
            gain: gain.min(100),
            bass: bass.min(100),
            mid: mid.min(100),
            treble: treble.min(100),
            presence: presence.min(100),
            volume: volume.min(100),
            enabled: true,
        }
    }

    pub fn set_model(&mut self, model: AmpModel) {
        self.model = model;
    }

    pub fn set_gain(&mut self, value: u8) {
        self.gain = value.min(100);
    }

    pub fn set_bass(&mut self, value: u8) {
        self.bass = value.min(100);
    }

    pub fn set_mid(&mut self, value: u8) {
        self.mid = value.min(100);
    }

    pub fn set_treble(&mut self, value: u8) {
        self.treble = value.min(100);
    }

    pub fn set_presence(&mut self, value: u8) {
        self.presence = value.min(100);
    }

    pub fn set_volume(&mut self, value: u8) {
        self.volume = value.min(100);
    }

    pub fn get_model(&self) -> &AmpModel {
        &self.model
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Effect for Amp {
    fn effect_type(&self) -> EffectType {
        self.model.to_effect_type()
    }

    fn name(&self) -> &str {
        match self.model {
            AmpModel::Clean => "Clean Amp",
            AmpModel::Crunch => "Crunch Amp",
            AmpModel::Lead => "Lead Amp",
            AmpModel::Bass => "Bass Amp",
            AmpModel::Acoustic => "Acoustic Amp",
            AmpModel::Vintage => "Vintage Amp",
            AmpModel::Modern => "Modern Amp",
            AmpModel::British => "British Amp",
            AmpModel::American => "American Amp",
            AmpModel::Metal => "Metal Amp",
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                name: "gain".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 30,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "bass".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "mid".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "treble".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "presence".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "volume".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 70,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "enabled".to_string(),
                min_value: 0,
                max_value: 1,
                default_value: 1,
                step: 1,
                unit: None,
            },
        ]
    }

    fn set_parameter(&mut self, param: &str, value: u8) -> Result<(), EffectError> {
        match param {
            "gain" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "gain must be 0-100".to_string(),
                    ));
                }
                self.gain = value;
            }
            "bass" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "bass must be 0-100".to_string(),
                    ));
                }
                self.bass = value;
            }
            "mid" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "mid must be 0-100".to_string(),
                    ));
                }
                self.mid = value;
            }
            "treble" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "treble must be 0-100".to_string(),
                    ));
                }
                self.treble = value;
            }
            "presence" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "presence must be 0-100".to_string(),
                    ));
                }
                self.presence = value;
            }
            "volume" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "volume must be 0-100".to_string(),
                    ));
                }
                self.volume = value;
            }
            "enabled" => {
                if value > 1 {
                    return Err(EffectError::ValueOutOfRange(
                        "enabled must be 0 or 1".to_string(),
                    ));
                }
                self.enabled = value == 1;
            }
            _ => return Err(EffectError::InvalidParameter(param.to_string())),
        }
        Ok(())
    }

    fn get_parameter(&self, param: &str) -> Option<u8> {
        match param {
            "gain" => Some(self.gain),
            "bass" => Some(self.bass),
            "mid" => Some(self.mid),
            "treble" => Some(self.treble),
            "presence" => Some(self.presence),
            "volume" => Some(self.volume),
            "enabled" => Some(if self.enabled { 1 } else { 0 }),
            _ => None,
        }
    }

    fn to_sysex(&self, device_type: DeviceType) -> Result<Vec<u8>, EffectError> {
        use crate::DeviceFactory;
        let device = DeviceFactory::create_device(device_type);
        let effect_id = device
            .effect_id(self.effect_type())
            .ok_or(EffectError::UnsupportedEffect(self.effect_type()))?;

        let mut sysex = device.sysex_header();
        sysex.extend(&[
            0x05, // Amp parameter change command (different from effects)
            effect_id,
            if self.enabled { 0x01 } else { 0x00 }, // On/Off
            self.gain,                              // Gain
            self.bass,                              // Bass
            self.mid,                               // Mid
            self.treble,                            // Treble
            self.presence,                          // Presence
            self.volume,                            // Volume
            0xF7,                                   // SysEx end
        ]);

        Ok(sysex)
    }

    fn from_sysex(data: &[u8], device_type: DeviceType) -> Result<Self, EffectError> {
        if data.len() < 12 {
            return Err(EffectError::InvalidMidiData);
        }

        use crate::DeviceFactory;
        let device = DeviceFactory::create_device(device_type);
        let header = device.sysex_header();

        // Verify header
        if !data.starts_with(&header) {
            return Err(EffectError::InvalidMidiData);
        }

        let enabled = data[header.len() + 2] == 0x01;
        let effect_id = data[header.len() + 1];

        // Determine amp model from effect ID
        let model = match effect_id {
            0x00 => AmpModel::Clean,
            0x01 => AmpModel::Crunch,
            0x02 => AmpModel::Lead,
            0x03 => AmpModel::Bass,
            0x04 => AmpModel::Acoustic,
            _ => AmpModel::Clean,
        };

        let gain = data[header.len() + 3].min(100);
        let bass = data[header.len() + 4].min(100);
        let mid = data[header.len() + 5].min(100);
        let treble = data[header.len() + 6].min(100);
        let presence = data[header.len() + 7].min(100);
        let volume = data[header.len() + 8].min(100);

        Ok(Self {
            model,
            gain,
            bass,
            mid,
            treble,
            presence,
            volume,
            enabled,
        })
    }
}

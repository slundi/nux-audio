// Compressor effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone)]
pub struct Compressor {
    threshold: u8, // 0-100 (compression threshold)
    ratio: u8,     // 0-100 (compression ratio)
    attack: u8,    // 0-100 (attack time)
    release: u8,   // 0-100 (release time)
    makeup: u8,    // 0-100 (makeup gain)
    enabled: bool,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            threshold: 60,
            ratio: 40,
            attack: 20,
            release: 50,
            makeup: 30,
            enabled: false,
        }
    }
}

impl Compressor {
    pub fn new(threshold: u8, ratio: u8, attack: u8, release: u8, makeup: u8) -> Self {
        Self {
            threshold: threshold.min(100),
            ratio: ratio.min(100),
            attack: attack.min(100),
            release: release.min(100),
            makeup: makeup.min(100),
            enabled: true,
        }
    }

    pub fn set_threshold(&mut self, value: u8) {
        self.threshold = value.min(100);
    }

    pub fn set_ratio(&mut self, value: u8) {
        self.ratio = value.min(100);
    }

    pub fn set_attack(&mut self, value: u8) {
        self.attack = value.min(100);
    }

    pub fn set_release(&mut self, value: u8) {
        self.release = value.min(100);
    }

    pub fn set_makeup(&mut self, value: u8) {
        self.makeup = value.min(100);
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

impl Effect for Compressor {
    fn effect_type(&self) -> EffectType {
        EffectType::Compressor
    }

    fn name(&self) -> &str {
        "Compressor"
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                name: "threshold".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 60,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "ratio".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 40,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "attack".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 20,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "release".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "makeup".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 30,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "enabled".to_string(),
                min_value: 0,
                max_value: 1,
                default_value: 0,
                step: 1,
                unit: None,
            },
        ]
    }

    fn set_parameter(&mut self, param: &str, value: u8) -> Result<(), EffectError> {
        match param {
            "threshold" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "threshold must be 0-100".to_string(),
                    ));
                }
                self.threshold = value;
            }
            "ratio" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "ratio must be 0-100".to_string(),
                    ));
                }
                self.ratio = value;
            }
            "attack" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "attack must be 0-100".to_string(),
                    ));
                }
                self.attack = value;
            }
            "release" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "release must be 0-100".to_string(),
                    ));
                }
                self.release = value;
            }
            "makeup" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "makeup must be 0-100".to_string(),
                    ));
                }
                self.makeup = value;
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
            "threshold" => Some(self.threshold),
            "ratio" => Some(self.ratio),
            "attack" => Some(self.attack),
            "release" => Some(self.release),
            "makeup" => Some(self.makeup),
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
            0x06, // Effect parameter change command
            effect_id,
            if self.enabled { 0x01 } else { 0x00 }, // On/Off
            self.threshold,                         // Threshold
            self.ratio,                             // Ratio
            self.attack,                            // Attack
            self.release,                           // Release
            self.makeup,                            // Makeup
            0xF7,                                   // SysEx end
        ]);

        Ok(sysex)
    }

    fn from_sysex(data: &[u8], device_type: DeviceType) -> Result<Self, EffectError> {
        if data.len() < 11 {
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
        let threshold = data[header.len() + 3].min(100);
        let ratio = data[header.len() + 4].min(100);
        let attack = data[header.len() + 5].min(100);
        let release = data[header.len() + 6].min(100);
        let makeup = data[header.len() + 7].min(100);

        Ok(Self {
            threshold,
            ratio,
            attack,
            release,
            makeup,
            enabled,
        })
    }
}

// Chorus effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone)]
pub struct Chorus {
    rate: u8,     // 0-100 (LFO speed)
    depth: u8,    // 0-100 (modulation depth)
    feedback: u8, // 0-100 (feedback amount)
    mix: u8,      // 0-100 (wet/dry mix)
    enabled: bool,
}

impl Default for Chorus {
    fn default() -> Self {
        Self {
            rate: 30,
            depth: 50,
            feedback: 20,
            mix: 50,
            enabled: false,
        }
    }
}

impl Chorus {
    pub fn new(rate: u8, depth: u8, feedback: u8, mix: u8) -> Self {
        Self {
            rate: rate.min(100),
            depth: depth.min(100),
            feedback: feedback.min(100),
            mix: mix.min(100),
            enabled: true,
        }
    }

    pub fn set_rate(&mut self, value: u8) {
        self.rate = value.min(100);
    }

    pub fn set_depth(&mut self, value: u8) {
        self.depth = value.min(100);
    }

    pub fn set_feedback(&mut self, value: u8) {
        self.feedback = value.min(100);
    }

    pub fn set_mix(&mut self, value: u8) {
        self.mix = value.min(100);
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

impl Effect for Chorus {
    fn effect_type(&self) -> EffectType {
        EffectType::Chorus
    }

    fn name(&self) -> &str {
        "Chorus"
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                name: "rate".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 30,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "depth".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "feedback".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 20,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "mix".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
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
            "rate" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "rate must be 0-100".to_string(),
                    ));
                }
                self.rate = value;
            }
            "depth" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "depth must be 0-100".to_string(),
                    ));
                }
                self.depth = value;
            }
            "feedback" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "feedback must be 0-100".to_string(),
                    ));
                }
                self.feedback = value;
            }
            "mix" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "mix must be 0-100".to_string(),
                    ));
                }
                self.mix = value;
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
            "rate" => Some(self.rate),
            "depth" => Some(self.depth),
            "feedback" => Some(self.feedback),
            "mix" => Some(self.mix),
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
            self.rate,                              // Rate
            self.depth,                             // Depth
            self.feedback,                          // Feedback
            self.mix,                               // Mix
            0xF7,                                   // SysEx end
        ]);

        Ok(sysex)
    }

    fn from_sysex(data: &[u8], device_type: DeviceType) -> Result<Self, EffectError> {
        if data.len() < 10 {
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
        let rate = data[header.len() + 3].min(100);
        let depth = data[header.len() + 4].min(100);
        let feedback = data[header.len() + 5].min(100);
        let mix = data[header.len() + 6].min(100);

        Ok(Self {
            rate,
            depth,
            feedback,
            mix,
            enabled,
        })
    }
}

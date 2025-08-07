// Delay effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone)]
pub struct Delay {
    time: u8,     // 0-100 (delay time)
    feedback: u8, // 0-100 (feedback amount)
    mix: u8,      // 0-100 (wet/dry mix)
    tone: u8,     // 0-100 (tone control)
    enabled: bool,
}

impl Default for Delay {
    fn default() -> Self {
        Self {
            time: 40,
            feedback: 30,
            mix: 25,
            tone: 50,
            enabled: false,
        }
    }
}

impl Delay {
    pub fn new(time: u8, feedback: u8, mix: u8, tone: u8) -> Self {
        Self {
            time: time.min(100),
            feedback: feedback.min(100),
            mix: mix.min(100),
            tone: tone.min(100),
            enabled: true,
        }
    }

    pub fn set_time(&mut self, value: u8) {
        self.time = value.min(100);
    }

    pub fn set_feedback(&mut self, value: u8) {
        self.feedback = value.min(100);
    }

    pub fn set_mix(&mut self, value: u8) {
        self.mix = value.min(100);
    }

    pub fn set_tone(&mut self, value: u8) {
        self.tone = value.min(100);
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

impl Effect for Delay {
    fn effect_type(&self) -> EffectType {
        EffectType::Delay
    }

    fn name(&self) -> &str {
        "Delay"
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                name: "time".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 40,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "feedback".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 30,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "mix".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 25,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "tone".to_string(),
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
            "time" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "time must be 0-100".to_string(),
                    ));
                }
                self.time = value;
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
            "tone" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "tone must be 0-100".to_string(),
                    ));
                }
                self.tone = value;
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
            "time" => Some(self.time),
            "feedback" => Some(self.feedback),
            "mix" => Some(self.mix),
            "tone" => Some(self.tone),
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
            self.time,                              // Time
            self.feedback,                          // Feedback
            self.mix,                               // Mix
            self.tone,                              // Tone
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
        let time = data[header.len() + 3].min(100);
        let feedback = data[header.len() + 4].min(100);
        let mix = data[header.len() + 5].min(100);
        let tone = data[header.len() + 6].min(100);

        Ok(Self {
            time,
            feedback,
            mix,
            tone,
            enabled,
        })
    }
}

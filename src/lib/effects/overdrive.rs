// Overdrive effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone)]
pub struct Overdrive {
    drive: u8, // 0-100
    tone: u8,  // 0-100
    level: u8, // 0-100
    enabled: bool,
}

impl Default for Overdrive {
    fn default() -> Self {
        Self {
            drive: 50,
            tone: 50,
            level: 50,
            enabled: false,
        }
    }
}

impl Overdrive {
    pub fn new(drive: u8, tone: u8, level: u8) -> Self {
        Self {
            drive: drive.min(100),
            tone: tone.min(100),
            level: level.min(100),
            enabled: true,
        }
    }

    pub fn set_drive(&mut self, value: u8) {
        self.drive = value.min(100);
    }

    pub fn set_tone(&mut self, value: u8) {
        self.tone = value.min(100);
    }

    pub fn set_level(&mut self, value: u8) {
        self.level = value.min(100);
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

impl Effect for Overdrive {
    fn effect_type(&self) -> EffectType {
        EffectType::Overdrive
    }

    fn name(&self) -> &str {
        "Overdrive"
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                name: "drive".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
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
                name: "level".to_string(),
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
            "drive" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "drive must be 0-100".to_string(),
                    ));
                }
                self.drive = value;
            }
            "tone" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "tone must be 0-100".to_string(),
                    ));
                }
                self.tone = value;
            }
            "level" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "level must be 0-100".to_string(),
                    ));
                }
                self.level = value;
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
            "drive" => Some(self.drive),
            "tone" => Some(self.tone),
            "level" => Some(self.level),
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
            self.drive,                             // Drive (0-100 mapped to 0-127)
            self.tone,                              // Tone
            self.level,                             // Level
            0xF7,                                   // SysEx end
        ]);

        Ok(sysex)
    }

    fn from_sysex(data: &[u8], device_type: DeviceType) -> Result<Self, EffectError> {
        if data.len() < 9 {
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
        let drive = data[header.len() + 3].min(100);
        let tone = data[header.len() + 4].min(100);
        let level = data[header.len() + 5].min(100);

        Ok(Self {
            drive,
            tone,
            level,
            enabled,
        })
    }
}

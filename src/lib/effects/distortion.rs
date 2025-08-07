// Distortion effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone, PartialEq)]
pub enum DistortionType {
    Standard,
    Vintage,
    Modern,
    Fuzz,
    Metal,
}

impl DistortionType {
    fn to_u8(&self) -> u8 {
        match self {
            DistortionType::Standard => 0,
            DistortionType::Vintage => 1,
            DistortionType::Modern => 2,
            DistortionType::Fuzz => 3,
            DistortionType::Metal => 4,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            0 => DistortionType::Standard,
            1 => DistortionType::Vintage,
            2 => DistortionType::Modern,
            3 => DistortionType::Fuzz,
            4 => DistortionType::Metal,
            _ => DistortionType::Standard,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Distortion {
    distortion_type: DistortionType,
    drive: u8, // 0-100
    tone: u8,  // 0-100
    level: u8, // 0-100
    gate: u8,  // 0-100 (noise gate threshold)
    enabled: bool,
}

impl Default for Distortion {
    fn default() -> Self {
        Self {
            distortion_type: DistortionType::Standard,
            drive: 60,
            tone: 50,
            level: 50,
            gate: 10,
            enabled: false,
        }
    }
}

impl Distortion {
    pub fn new(distortion_type: DistortionType, drive: u8, tone: u8, level: u8, gate: u8) -> Self {
        Self {
            distortion_type,
            drive: drive.min(100),
            tone: tone.min(100),
            level: level.min(100),
            gate: gate.min(100),
            enabled: true,
        }
    }

    pub fn set_type(&mut self, distortion_type: DistortionType) {
        self.distortion_type = distortion_type;
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

    pub fn set_gate(&mut self, value: u8) {
        self.gate = value.min(100);
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

    pub fn get_type(&self) -> &DistortionType {
        &self.distortion_type
    }
}

impl Effect for Distortion {
    fn effect_type(&self) -> EffectType {
        match self.distortion_type {
            DistortionType::Fuzz => EffectType::Fuzz,
            _ => EffectType::Distortion,
        }
    }

    fn name(&self) -> &str {
        match self.distortion_type {
            DistortionType::Standard => "Distortion",
            DistortionType::Vintage => "Vintage Distortion",
            DistortionType::Modern => "Modern Distortion",
            DistortionType::Fuzz => "Fuzz",
            DistortionType::Metal => "Metal Distortion",
        }
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter {
                name: "type".to_string(),
                min_value: 0,
                max_value: 4,
                default_value: 0,
                step: 1,
                unit: None,
            },
            Parameter {
                name: "drive".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 60,
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
                name: "gate".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 10,
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
            "type" => {
                if value > 4 {
                    return Err(EffectError::ValueOutOfRange("type must be 0-4".to_string()));
                }
                self.distortion_type = DistortionType::from_u8(value);
            }
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
            "gate" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "gate must be 0-100".to_string(),
                    ));
                }
                self.gate = value;
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
            "type" => Some(self.distortion_type.to_u8()),
            "drive" => Some(self.drive),
            "tone" => Some(self.tone),
            "level" => Some(self.level),
            "gate" => Some(self.gate),
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
            self.distortion_type.to_u8(),           // Type
            self.drive,                             // Drive
            self.tone,                              // Tone
            self.level,                             // Level
            self.gate,                              // Gate
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
        let distortion_type = DistortionType::from_u8(data[header.len() + 3]);
        let drive = data[header.len() + 4].min(100);
        let tone = data[header.len() + 5].min(100);
        let level = data[header.len() + 6].min(100);
        let gate = data[header.len() + 7].min(100);

        Ok(Self {
            distortion_type,
            drive,
            tone,
            level,
            gate,
            enabled,
        })
    }
}

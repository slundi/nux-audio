// Reverb effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone, PartialEq)]
pub enum ReverbType {
    Room,
    Hall,
    Plate,
    Spring,
    Cathedral,
}

impl ReverbType {
    fn to_u8(&self) -> u8 {
        match self {
            ReverbType::Room => 0,
            ReverbType::Hall => 1,
            ReverbType::Plate => 2,
            ReverbType::Spring => 3,
            ReverbType::Cathedral => 4,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            0 => ReverbType::Room,
            1 => ReverbType::Hall,
            2 => ReverbType::Plate,
            3 => ReverbType::Spring,
            4 => ReverbType::Cathedral,
            _ => ReverbType::Room,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reverb {
    reverb_type: ReverbType,
    size: u8,     // 0-100 (room size/decay time)
    damping: u8,  // 0-100 (high frequency damping)
    mix: u8,      // 0-100 (wet/dry mix)
    predelay: u8, // 0-100 (predelay time)
    enabled: bool,
}

impl Default for Reverb {
    fn default() -> Self {
        Self {
            reverb_type: ReverbType::Room,
            size: 50,
            damping: 50,
            mix: 30,
            predelay: 20,
            enabled: false,
        }
    }
}

impl Reverb {
    pub fn new(reverb_type: ReverbType, size: u8, damping: u8, mix: u8, predelay: u8) -> Self {
        Self {
            reverb_type,
            size: size.min(100),
            damping: damping.min(100),
            mix: mix.min(100),
            predelay: predelay.min(100),
            enabled: true,
        }
    }

    pub fn set_type(&mut self, reverb_type: ReverbType) {
        self.reverb_type = reverb_type;
    }

    pub fn set_size(&mut self, value: u8) {
        self.size = value.min(100);
    }

    pub fn set_damping(&mut self, value: u8) {
        self.damping = value.min(100);
    }

    pub fn set_mix(&mut self, value: u8) {
        self.mix = value.min(100);
    }

    pub fn set_predelay(&mut self, value: u8) {
        self.predelay = value.min(100);
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

    pub fn get_type(&self) -> &ReverbType {
        &self.reverb_type
    }
}

impl Effect for Reverb {
    fn effect_type(&self) -> EffectType {
        EffectType::Reverb
    }

    fn name(&self) -> &str {
        "Reverb"
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
                name: "size".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "damping".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "mix".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 30,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "predelay".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 20,
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
                self.reverb_type = ReverbType::from_u8(value);
            }
            "size" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "size must be 0-100".to_string(),
                    ));
                }
                self.size = value;
            }
            "damping" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "damping must be 0-100".to_string(),
                    ));
                }
                self.damping = value;
            }
            "mix" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "mix must be 0-100".to_string(),
                    ));
                }
                self.mix = value;
            }
            "predelay" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "predelay must be 0-100".to_string(),
                    ));
                }
                self.predelay = value;
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
            "type" => Some(self.reverb_type.to_u8()),
            "size" => Some(self.size),
            "damping" => Some(self.damping),
            "mix" => Some(self.mix),
            "predelay" => Some(self.predelay),
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
            self.reverb_type.to_u8(),               // Type
            self.size,                              // Size
            self.damping,                           // Damping
            self.mix,                               // Mix
            self.predelay,                          // Predelay
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
        let reverb_type = ReverbType::from_u8(data[header.len() + 3]);
        let size = data[header.len() + 4].min(100);
        let damping = data[header.len() + 5].min(100);
        let mix = data[header.len() + 6].min(100);
        let predelay = data[header.len() + 7].min(100);

        Ok(Self {
            reverb_type,
            size,
            damping,
            mix,
            predelay,
            enabled,
        })
    }
}

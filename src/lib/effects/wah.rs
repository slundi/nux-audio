// effects/wah.rs - Wah effect implementation
use crate::{DeviceType, Effect, EffectError, EffectType, Parameter};

#[derive(Debug, Clone, PartialEq)]
pub enum WahType {
    Classic,  // Classic wah sound
    Vintage,  // Vintage wah simulation
    Modern,   // Modern wah with extended range
    AutoWah,  // Auto wah (envelope follower)
    TouchWah, // Touch-sensitive wah
}

impl WahType {
    fn to_u8(&self) -> u8 {
        match self {
            WahType::Classic => 0,
            WahType::Vintage => 1,
            WahType::Modern => 2,
            WahType::AutoWah => 3,
            WahType::TouchWah => 4,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            0 => WahType::Classic,
            1 => WahType::Vintage,
            2 => WahType::Modern,
            3 => WahType::AutoWah,
            4 => WahType::TouchWah,
            _ => WahType::Classic,
        }
    }

    fn to_effect_type(&self) -> EffectType {
        match self {
            WahType::AutoWah => EffectType::AutoWah,
            _ => EffectType::Wah,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Wah {
    wah_type: WahType,
    position: u8,    // 0-100 (wah pedal position, heel to toe)
    range: u8,       // 0-100 (frequency range of the wah)
    resonance: u8,   // 0-100 (resonance/Q factor)
    sensitivity: u8, // 0-100 (for auto wah - input sensitivity)
    mix: u8,         // 0-100 (wet/dry mix)
    enabled: bool,
}

impl Default for Wah {
    fn default() -> Self {
        Self {
            wah_type: WahType::Classic,
            position: 50,
            range: 70,
            resonance: 60,
            sensitivity: 50,
            mix: 100, // Usually 100% wet for wah
            enabled: false,
        }
    }
}

impl Wah {
    pub fn new(
        wah_type: WahType,
        position: u8,
        range: u8,
        resonance: u8,
        sensitivity: u8,
        mix: u8,
    ) -> Self {
        Self {
            wah_type,
            position: position.min(100),
            range: range.min(100),
            resonance: resonance.min(100),
            sensitivity: sensitivity.min(100),
            mix: mix.min(100),
            enabled: true,
        }
    }

    pub fn set_type(&mut self, wah_type: WahType) {
        self.wah_type = wah_type;
    }

    pub fn set_position(&mut self, value: u8) {
        self.position = value.min(100);
    }

    pub fn set_range(&mut self, value: u8) {
        self.range = value.min(100);
    }

    pub fn set_resonance(&mut self, value: u8) {
        self.resonance = value.min(100);
    }

    pub fn set_sensitivity(&mut self, value: u8) {
        self.sensitivity = value.min(100);
    }

    pub fn set_mix(&mut self, value: u8) {
        self.mix = value.min(100);
    }

    /// Heel position (low frequency)
    pub fn heel(&mut self) {
        self.position = 0;
    }

    /// Toe position (high frequency)
    pub fn toe(&mut self) {
        self.position = 100;
    }

    /// Middle position
    pub fn middle(&mut self) {
        self.position = 50;
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

    pub fn get_type(&self) -> &WahType {
        &self.wah_type
    }
}

impl Effect for Wah {
    fn effect_type(&self) -> EffectType {
        self.wah_type.to_effect_type()
    }

    fn name(&self) -> &str {
        match self.wah_type {
            WahType::Classic => "Classic Wah",
            WahType::Vintage => "Vintage Wah",
            WahType::Modern => "Modern Wah",
            WahType::AutoWah => "Auto Wah",
            WahType::TouchWah => "Touch Wah",
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
                name: "position".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 50,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "range".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 70,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "resonance".to_string(),
                min_value: 0,
                max_value: 100,
                default_value: 60,
                step: 1,
                unit: Some("%".to_string()),
            },
            Parameter {
                name: "sensitivity".to_string(),
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
                default_value: 100,
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
                self.wah_type = WahType::from_u8(value);
            }
            "position" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "position must be 0-100".to_string(),
                    ));
                }
                self.position = value;
            }
            "range" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "range must be 0-100".to_string(),
                    ));
                }
                self.range = value;
            }
            "resonance" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "resonance must be 0-100".to_string(),
                    ));
                }
                self.resonance = value;
            }
            "sensitivity" => {
                if value > 100 {
                    return Err(EffectError::ValueOutOfRange(
                        "sensitivity must be 0-100".to_string(),
                    ));
                }
                self.sensitivity = value;
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
            "type" => Some(self.wah_type.to_u8()),
            "position" => Some(self.position),
            "range" => Some(self.range),
            "resonance" => Some(self.resonance),
            "sensitivity" => Some(self.sensitivity),
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
            self.wah_type.to_u8(),                  // Type
            self.position,                          // Position
            self.range,                             // Range
            self.resonance,                         // Resonance
            self.sensitivity,                       // Sensitivity
            self.mix,                               // Mix
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

        todo!();
    }
}

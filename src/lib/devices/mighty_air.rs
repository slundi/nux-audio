use std::collections::HashMap;

use super::{BaseNuxDevice, DeviceInfo, DeviceModel, NuxDevice, MIGHTY_AIR_DEVICE_ID};
use crate::{poc::{Effect, EffectParameter, EffectType, MidiCommand, Preset}};

#[derive(Debug, Clone)]
pub struct MightyAir {
    base: BaseNuxDevice,
}

impl Default for MightyAir {
    fn default() -> Self {
        let info = DeviceInfo {
            model: DeviceModel::MightyAir,
            firmware_version: "1.0.0".to_string(),
            device_id: MIGHTY_AIR_DEVICE_ID,
            bluetooth_name: "MIGHTY AIR".to_string(),
            supports_dual_bluetooth: true,
            max_presets: 8,
        };

        let mut device = Self {
            base: BaseNuxDevice::new(info),
        };

        // Mighty Air effects (wireless guitar system with effects)
        device.base.effects = vec![
            Effect {
                effect_type: EffectType::Gate,
                name: "Gate".to_string(),
                enabled: false,
                parameters: HashMap::from([(
                    "threshold".to_string(),
                    EffectParameter {
                        name: "Threshold".to_string(),
                        value: 64,
                        min_value: 0,
                        max_value: 127,
                        default_value: 64,
                    },
                )]),
            },
            Effect {
                effect_type: EffectType::Compressor,
                name: "Compressor".to_string(),
                enabled: false,
                parameters: HashMap::from([
                    (
                        "sustain".to_string(),
                        EffectParameter {
                            name: "Sustain".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "level".to_string(),
                        EffectParameter {
                            name: "Level".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                ]),
            },
            Effect {
                effect_type: EffectType::Overdrive,
                name: "Overdrive".to_string(),
                enabled: false,
                parameters: HashMap::from([
                    (
                        "drive".to_string(),
                        EffectParameter {
                            name: "Drive".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "tone".to_string(),
                        EffectParameter {
                            name: "Tone".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "level".to_string(),
                        EffectParameter {
                            name: "Level".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                ]),
            },
            Effect {
                effect_type: EffectType::Distortion,
                name: "Distortion".to_string(),
                enabled: false,
                parameters: HashMap::from([
                    (
                        "gain".to_string(),
                        EffectParameter {
                            name: "Gain".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "tone".to_string(),
                        EffectParameter {
                            name: "Tone".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "level".to_string(),
                        EffectParameter {
                            name: "Level".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                ]),
            },
            Effect {
                effect_type: EffectType::Chorus,
                name: "Chorus".to_string(),
                enabled: false,
                parameters: HashMap::from([
                    (
                        "rate".to_string(),
                        EffectParameter {
                            name: "Rate".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "depth".to_string(),
                        EffectParameter {
                            name: "Depth".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                ]),
            },
            Effect {
                effect_type: EffectType::Delay,
                name: "Delay".to_string(),
                enabled: false,
                parameters: HashMap::from([
                    (
                        "time".to_string(),
                        EffectParameter {
                            name: "Time".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "feedback".to_string(),
                        EffectParameter {
                            name: "Feedback".to_string(),
                            value: 32,
                            min_value: 0,
                            max_value: 127,
                            default_value: 32,
                        },
                    ),
                    (
                        "level".to_string(),
                        EffectParameter {
                            name: "Level".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                ]),
            },
            Effect {
                effect_type: EffectType::Reverb,
                name: "Reverb".to_string(),
                enabled: false,
                parameters: HashMap::from([
                    (
                        "decay".to_string(),
                        EffectParameter {
                            name: "Decay".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "level".to_string(),
                        EffectParameter {
                            name: "Level".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                ]),
            },
            Effect {
                effect_type: EffectType::Tuner,
                name: "Tuner".to_string(),
                enabled: false,
                parameters: HashMap::new(),
            },
        ];

        device
    }
}

impl NuxDevice for MightyAir {
    fn get_device_info(&self) -> &DeviceInfo {
        self.base.get_device_info()
    }

    fn get_presets(&self) -> &[Preset] {
        self.base.get_presets()
    }

    fn get_current_preset(&self) -> Option<&Preset> {
        self.base.get_current_preset()
    }

    fn set_preset(&mut self, preset_id: u8) -> Result<(), String> {
        self.base.set_preset(preset_id)
    }

    fn get_effects(&self) -> &[Effect] {
        self.base.get_effects()
    }

    fn toggle_effect(&mut self, effect_id: u8) -> Result<(), String> {
        self.base.toggle_effect(effect_id)
    }

    fn set_parameter(&mut self, effect_id: u8, param_name: &str, value: u8) -> Result<(), String> {
        self.base.set_parameter(effect_id, param_name, value)
    }

    fn generate_sysex(&self, command: &MidiCommand) -> Result<Vec<u8>, String> {
        self.base.generate_sysex(command)
    }

    fn parse_sysex(&mut self, data: &[u8]) -> Result<Option<MidiCommand>, String> {
        self.base.parse_sysex(data)
    }

    fn get_bluetooth_names(&self) -> Vec<String> {
        self.base.get_bluetooth_names()
    }
}

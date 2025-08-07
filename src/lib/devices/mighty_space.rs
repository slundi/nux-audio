#[derive(Debug, Clone)]
pub struct MightySpace {
    base: BaseNuxDevice,
}

impl MightySpace {
    pub fn new() -> Self {
        let info = DeviceInfo {
            model: DeviceModel::MightySpace,
            firmware_version: "1.0.0".to_string(),
            device_id: MIGHTY_SPACE_DEVICE_ID,
            bluetooth_name: "MIGHTY SPACE".to_string(),
            supports_dual_bluetooth: false,
            max_presets: 8,
        };

        let mut device = Self {
            base: BaseNuxDevice::new(info),
        };

        // Mighty Space effects (wireless earphone amplifier)
        device.base.effects = vec![
            Effect {
                effect_type: EffectType::Amp,
                name: "Amp".to_string(),
                enabled: true,
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
                        "bass".to_string(),
                        EffectParameter {
                            name: "Bass".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "mid".to_string(),
                        EffectParameter {
                            name: "Mid".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "treble".to_string(),
                        EffectParameter {
                            name: "Treble".to_string(),
                            value: 64,
                            min_value: 0,
                            max_value: 127,
                            default_value: 64,
                        },
                    ),
                    (
                        "volume".to_string(),
                        EffectParameter {
                            name: "Volume".to_string(),
                            value: 100,
                            min_value: 0,
                            max_value: 127,
                            default_value: 100,
                        },
                    ),
                ]),
            },
            Effect {
                effect_type: EffectType::Cabinet,
                name: "Cabinet".to_string(),
                enabled: true,
                parameters: HashMap::from([(
                    "room_size".to_string(),
                    EffectParameter {
                        name: "Room Size".to_string(),
                        value: 64,
                        min_value: 0,
                        max_value: 127,
                        default_value: 64,
                    },
                )]),
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
        ];

        device
    }
}

impl NuxDevice for MightySpace {
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

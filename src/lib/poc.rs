use std::collections::HashMap;

// MIDI constants
pub const NUX_MANUFACTURER_ID: [u8; 3] = [0x00, 0x21, 0x21]; // Example - actual ID may vary

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectType {
    Gate,
    Compressor,
    Overdrive,
    Distortion,
    Fuzz,
    Amp,
    Cabinet,
    Eq,
    Chorus,
    Phaser,
    Flanger,
    Tremolo,
    Reverb,
    Delay,
    ModDelay,
    TapeDelay,
    Looper,
    Tuner,
}

#[derive(Debug, Clone)]
pub struct EffectParameter {
    pub name: String,
    pub value: u8, // 0-127
    pub min_value: u8,
    pub max_value: u8,
    pub default_value: u8,
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub effect_type: EffectType,
    pub name: String,
    pub enabled: bool,
    pub parameters: HashMap<String, EffectParameter>,
}

#[derive(Debug, Clone)]
pub struct Preset {
    pub id: u8,
    pub name: String,
    pub effects: Vec<Effect>,
    pub amp_model: String,
    pub cabinet_ir: String,
    pub bpm: u8,
}

#[derive(Debug, Clone)]
pub enum MidiCommand {
    PresetChange(u8),
    ParameterChange {
        effect_id: u8,
        param_id: u8,
        value: u8,
    },
    EffectToggle {
        effect_id: u8,
        enabled: bool,
    },
    TunerOn,
    TunerOff,
    TapTempo,
    LooperRecord,
    LooperPlay,
    LooperStop,
    DeviceInfo,
    Custom(Vec<u8>),
}

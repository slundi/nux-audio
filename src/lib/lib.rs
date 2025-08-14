// // Main NUX Audio Effects Library
// use std::collections::HashMap;

pub mod devices;
// pub mod effects;
pub mod poc;
pub mod midi;
pub mod nux_sys_ex_commands;
// pub mod presets;
pub mod string;

// pub use effects::*;
pub use poc::*;
pub use midi::*;
// pub use presets::*;

// /// Main trait for all NUX audio effects
// pub trait Effect: std::fmt::Debug + Clone{
//     /// Get the effect type identifier
//     fn effect_type(&self) -> EffectType;

//     /// Get the effect name
//     fn name(&self) -> &str;

//     /// Get all available parameters for this effect
//     fn parameters(&self) -> Vec<Parameter>;

//     /// Set a parameter value
//     fn set_parameter(&mut self, param: &str, value: u8) -> Result<(), EffectError>;

//     /// Get a parameter value
//     fn get_parameter(&self, param: &str) -> Option<u8>;

//     /// Generate MIDI SysEx message for this effect
//     fn to_sysex(&self, device_type: DeviceType) -> Result<Vec<u8>, EffectError>;

//     /// Create effect from MIDI SysEx data
//     fn from_sysex(data: &[u8], device_type: DeviceType) -> Result<Self, EffectError>
//     where
//         Self: Sized;
// }

// /// Effect parameter definition
// #[derive(Debug, Clone, PartialEq)]
// pub struct Parameter {
//     pub name: String,
//     pub min_value: u8,
//     pub max_value: u8,
//     pub default_value: u8,
//     pub step: u8,
//     pub unit: Option<String>,
// }

// /// Effect types supported by NUX devices
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum EffectType {
//     // Amp models
//     AmpClean,
//     AmpCrunch,
//     AmpLead,
//     AmpBass,
//     AmpAcoustic,

//     // Distortion/Overdrive
//     Overdrive,
//     Distortion,
//     Fuzz,
//     TubeScreamer,
//     MetalZone,

//     // Modulation
//     Chorus,
//     Flanger,
//     Phaser,
//     Tremolo,
//     Vibrato,
//     Rotary,

//     // Time-based
//     Delay,
//     Echo,
//     Reverb,

//     // Filter
//     Wah,
//     AutoWah,
//     Compressor,

//     // Utility
//     Tuner,
//     NoiseGate,
//     Boost,
// }

// /// Effect processing errors
// #[derive(Debug, thiserror::Error)]
// pub enum EffectError {
//     #[error("Invalid parameter: {0}")]
//     InvalidParameter(String),

//     #[error("Parameter value out of range: {0}")]
//     ValueOutOfRange(String),

//     #[error("Unsupported effect type: {0:?}")]
//     UnsupportedEffect(EffectType),

//     #[error("Invalid MIDI data")]
//     InvalidMidiData,

//     #[error("Device not supported")]
//     DeviceNotSupported,
// }

// /// Preset management
// #[derive(Debug, Clone)]
// pub struct Preset {
//     pub name: String,
//     pub effects: HashMap<EffectType, Box<dyn Effect>>,
//     pub device_type: DeviceType,
// }

// impl Preset {
//     pub fn new(name: String, device_type: DeviceType) -> Self {
//         Self {
//             name,
//             effects: HashMap::new(),
//             device_type,
//         }
//     }

//     pub fn add_effect<E: Effect + 'static>(&mut self, effect: E) {
//         self.effects.insert(effect.effect_type(), Box::new(effect));
//     }

//     pub fn to_sysex(&self) -> Result<Vec<u8>, EffectError> {
//         let mut sysex_data = Vec::new();

//         for effect in self.effects.values() {
//             sysex_data.extend(effect.to_sysex(self.device_type)?);
//         }

//         Ok(sysex_data)
//     }
// }

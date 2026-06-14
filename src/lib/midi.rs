// MIDI utilities and SysEx helpers
// use crate::DeviceType;

/// Universal SysEx start byte (`0xF0`), required at the beginning of every SysEx message.
pub const SYSEX_START: u8 = 0xF0;
/// Universal SysEx end byte (`0xF7`), required at the end of every SysEx message.
pub const SYSEX_END: u8 = 0xF7;

// /// SysEx message builder for NUX devices
// pub struct SysExBuilder {
//     device_type: DeviceType,
//     data: Vec<u8>,
// }

// impl SysExBuilder {
//     pub fn new(device_type: DeviceType) -> Self {
//         use crate::DeviceFactory;
//         let device = DeviceFactory::create_device(device_type);

//         Self {
//             device_type,
//             data: device.sysex_header(),
//         }
//     }

//     /// Add a command byte
//     pub fn command(mut self, cmd: u8) -> Self {
//         self.data.push(cmd);
//         self
//     }

//     /// Add effect ID
//     pub fn effect_id(mut self, effect_id: u8) -> Self {
//         self.data.push(effect_id);
//         self
//     }

//     /// Add parameter value
//     pub fn parameter(mut self, value: u8) -> Self {
//         self.data.push(value);
//         self
//     }

//     /// Add multiple parameter values
//     pub fn parameters(mut self, values: &[u8]) -> Self {
//         self.data.extend_from_slice(values);
//         self
//     }

//     /// Build the final SysEx message
//     pub fn build(mut self) -> Vec<u8> {
//         self.data.push(0xF7); // SysEx end
//         self.data
//     }
// }

/// Common MIDI commands for NUX devices
pub mod commands {
    /// Switch to a different preset slot.
    pub const PRESET_CHANGE: u8 = 0x01;
    /// Toggle an effect slot on or off.
    pub const EFFECT_TOGGLE: u8 = 0x02;
    /// Set a parameter value for a standard effect slot.
    pub const EFFECT_PARAMETER: u8 = 0x07;
    /// Set a parameter value specifically for the amp slot.
    pub const AMP_PARAMETER: u8 = 0x05;
    /// Switch the model (type) within an effect slot.
    pub const EFFECT_CHANGE: u8 = 0x06;
    /// Enable the built-in chromatic tuner.
    pub const TUNER_ON: u8 = 0x10;
    /// Disable the built-in chromatic tuner.
    pub const TUNER_OFF: u8 = 0x11;
    /// Write the current edit buffer to a preset slot.
    pub const SAVE_PRESET: u8 = 0x20;
    /// Load a preset slot into the edit buffer.
    pub const LOAD_PRESET: u8 = 0x21;
}

/// Parameter IDs for common effect parameters
pub mod parameters {
    // Common parameters
    /// Effect bypass/enable state (0=off, non-zero=on).
    pub const ENABLED: u8 = 0x00;
    /// Output level of the effect.
    pub const LEVEL: u8 = 0x01;
    /// Drive/gain amount applied before the effect circuit.
    pub const DRIVE: u8 = 0x02;
    /// Tone / EQ brightness of the effect output.
    pub const TONE: u8 = 0x03;
    /// Wet/dry mix ratio.
    pub const MIX: u8 = 0x04;

    // Amp parameters
    /// Amp pre-gain (distortion amount).
    pub const AMP_GAIN: u8 = 0x10;
    /// Amp bass EQ band.
    pub const AMP_BASS: u8 = 0x11;
    /// Amp mid EQ band.
    pub const AMP_MID: u8 = 0x12;
    /// Amp treble EQ band.
    pub const AMP_TREBLE: u8 = 0x13;
    /// Amp presence (high-frequency emphasis).
    pub const AMP_PRESENCE: u8 = 0x14;
    /// Amp master output volume.
    pub const AMP_VOLUME: u8 = 0x15;

    // Modulation parameters
    /// Modulation effect rate (speed).
    pub const MOD_RATE: u8 = 0x20;
    /// Modulation effect depth (intensity).
    pub const MOD_DEPTH: u8 = 0x21;
    /// Modulation feedback amount.
    pub const MOD_FEEDBACK: u8 = 0x22;

    // Time-based parameters
    /// Delay time in milliseconds.
    pub const DELAY_TIME: u8 = 0x30;
    /// Delay feedback (number of repeats).
    pub const DELAY_FEEDBACK: u8 = 0x31;
    /// Reverb room/hall size.
    pub const REVERB_SIZE: u8 = 0x32;
    /// Reverb high-frequency damping.
    pub const REVERB_DAMPING: u8 = 0x33;
}

/// Utility functions for MIDI data conversion
pub mod utils {
    /// Convert 0-100 range to MIDI 0-127 range
    pub fn to_midi_value(value: u8) -> u8 {
        ((value as u16 * 127) / 100) as u8
    }

    /// Convert MIDI 0-127 range to 0-100 range
    pub fn from_midi_value(value: u8) -> u8 {
        ((value as u16 * 100) / 127) as u8
    }

    /// Validate SysEx message checksum (if used by device)
    pub fn validate_checksum(data: &[u8]) -> bool {
        if data.len() < 3 {
            return false;
        }

        // Simple checksum validation for NUX devices
        let sum: u8 = data[1..data.len() - 2].iter().sum();
        let checksum = data[data.len() - 2];
        sum == checksum
    }

    /// Calculate checksum for SysEx message
    pub fn calculate_checksum(data: &[u8]) -> u8 {
        data[1..data.len() - 1].iter().sum()
    }
}

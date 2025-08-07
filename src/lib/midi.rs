// MIDI utilities and SysEx helpers
// use crate::DeviceType;
use midir::{MidiOutput, MidiOutputConnection};
use std::error::Error;

/// MIDI connection manager for NUX devices
pub struct MidiManager {
    output: MidiOutput,
    connection: Option<MidiOutputConnection>,
}

impl MidiManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let output = MidiOutput::new("NUX Audio MIDI Output")?;

        Ok(Self {
            output,
            connection: None,
        })
    }

    /// List available MIDI output ports
    pub fn list_ports(&self) -> Vec<(usize, String)> {
        let ports = self.output.ports();
        ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| self.output.port_name(port).ok().map(|name| (i, name)))
            .collect()
    }

    /// Connect to a MIDI port by index
    pub fn connect(&mut self, port_index: usize) -> Result<(), Box<dyn Error>> {
        let ports = self.output.ports();
        if port_index >= ports.len() {
            return Err("Port index out of range".into());
        }

        let port = &ports[port_index];
        let port_name = self.output.port_name(port)?;

        // Create a new output instance for the connection
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        let connection = output.connect(port, &format!("nux-audio-{}", port_name))?;

        self.connection = Some(connection);
        Ok(())
    }

    /// Send SysEx data to the connected device
    pub fn send_sysex(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        match &mut self.connection {
            Some(conn) => {
                conn.send(data)?;
                Ok(())
            }
            None => Err("No MIDI connection established".into()),
        }
    }

    // /// Send effect parameter change
    // pub fn send_effect_parameter(
    //     &mut self,
    //     device_type: DeviceType,
    //     effect_id: u8,
    //     param_id: u8,
    //     value: u8,
    // ) -> Result<(), Box<dyn Error>> {
    //     use crate::DeviceFactory;
    //     let device = DeviceFactory::create_device(device_type);

    //     let mut sysex = device.sysex_header();
    //     sysex.extend(&[
    //         0x07, // Parameter change command
    //         effect_id, param_id, value, 0xF7, // SysEx end
    //     ]);

    //     self.send_sysex(&sysex)
    // }

    // /// Send preset change command
    // pub fn send_preset_change(
    //     &mut self,
    //     device_type: DeviceType,
    //     preset_number: u8,
    // ) -> Result<(), Box<dyn Error>> {
    //     use crate::DeviceFactory;
    //     let device = DeviceFactory::create_device(device_type);

    //     let mut sysex = device.sysex_header();
    //     sysex.extend(&[
    //         0x01, // Preset change command
    //         preset_number,
    //         0xF7, // SysEx end
    //     ]);

    //     self.send_sysex(&sysex)
    // }

    /// Disconnect from the current MIDI device
    pub fn disconnect(&mut self) {
        self.connection = None;
    }
}

impl Drop for MidiManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}

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
    pub const PRESET_CHANGE: u8 = 0x01;
    pub const EFFECT_TOGGLE: u8 = 0x02;
    pub const EFFECT_PARAMETER: u8 = 0x07;
    pub const AMP_PARAMETER: u8 = 0x05;
    pub const EFFECT_CHANGE: u8 = 0x06;
    pub const TUNER_ON: u8 = 0x10;
    pub const TUNER_OFF: u8 = 0x11;
    pub const SAVE_PRESET: u8 = 0x20;
    pub const LOAD_PRESET: u8 = 0x21;
}

/// Parameter IDs for common effect parameters
pub mod parameters {
    // Common parameters
    pub const ENABLED: u8 = 0x00;
    pub const LEVEL: u8 = 0x01;
    pub const DRIVE: u8 = 0x02;
    pub const TONE: u8 = 0x03;
    pub const MIX: u8 = 0x04;

    // Amp parameters
    pub const AMP_GAIN: u8 = 0x10;
    pub const AMP_BASS: u8 = 0x11;
    pub const AMP_MID: u8 = 0x12;
    pub const AMP_TREBLE: u8 = 0x13;
    pub const AMP_PRESENCE: u8 = 0x14;
    pub const AMP_VOLUME: u8 = 0x15;

    // Modulation parameters
    pub const MOD_RATE: u8 = 0x20;
    pub const MOD_DEPTH: u8 = 0x21;
    pub const MOD_FEEDBACK: u8 = 0x22;

    // Time-based parameters
    pub const DELAY_TIME: u8 = 0x30;
    pub const DELAY_FEEDBACK: u8 = 0x31;
    pub const REVERB_SIZE: u8 = 0x32;
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

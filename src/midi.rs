use std::io::stdin;

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use nux_audio::{SYSEX_END, SYSEX_START};
use std::error::Error;

/// MIDI connection manager for NUX devices
pub struct MidiManager {
    input: MidiInput,
    output: MidiOutput,
    connection_in: Option<MidiInputConnection<()>>,
    connection_out: Option<MidiOutputConnection>,
}

impl MidiManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        let input = MidiInput::new("NUX Audio MIDI Input")?;

        Ok(Self {
            input,
            output,
            connection_in: None,
            connection_out: None,
        })
    }

    /// List available MIDI output ports
    pub fn list_output_ports(&self) -> Vec<(usize, String)> {
        let ports = self.output.ports();
        ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| self.output.port_name(port).ok().map(|name| (i, name)))
            .collect()
    }

    /// List available MIDI input ports
    pub fn list_input_ports(&self) -> Vec<(usize, String)> {
        let ports = self.input.ports();
        ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| self.input.port_name(port).ok().map(|name| (i, name)))
            .collect()
    }

    /// Connect to a MIDI port by index
    pub fn connect(&mut self, port_in: usize, port_out: usize) -> Result<(), Box<dyn Error>> {
        // check port range
        let ports_out = self.output.ports();
        if port_out >= ports_out.len() {
            return Err("Output port index out of range".into());
        }
        let ports_in = self.input.ports();
        if port_in >= ports_in.len() {
            return Err("Input port index out of range".into());
        }

        let port = &ports_out[port_out];
        let port_name = self.output.port_name(port)?;

        // Create a new output instance for the connection
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        let connection_out = output.connect(port, &format!("nux-audio-{}", port_name))?;
        self.connection_out = Some(connection_out);

        let port = &ports_in[port_in];
        let port_name = self.input.port_name(port)?;

        let input = MidiInput::new("NUX Audio MIDI Intput")?;
        // let connection_in = input.connect(port, &format!("nux-audio-{}", port_name), move |stamp, message, data| {
        //     println!("TODO: callback: {stamp}ms\n\tSent: {:?}\n\tResp: {:?}", message, data);
        // }, ())?;
        let connection_in = input.connect(
            port,
            &format!("nux-audio-{}", port_name),
            midi_input_callback,
            (),
        )?;

        self.connection_in = Some(connection_in);
        Ok(())
    }

    /// Send SysEx data to the connected device
    pub fn send_sysex(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut buffer = Vec::with_capacity(data.len() + 3);
        buffer.push(SYSEX_START);
        buffer.push(0x43);
        buffer.extend_from_slice(data);
        buffer.push(SYSEX_END);
        match &mut self.connection_out {
            Some(conn) => {
                conn.send(&buffer)?;
                Ok(())
            }
            None => Err("No MIDI connection established".into()),
        }
    }

    pub fn send_sysex_raw(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut buffer = Vec::with_capacity(data.len() + 2);
        buffer.push(SYSEX_START);
        buffer.extend_from_slice(data);
        buffer.push(SYSEX_END);
        match &mut self.connection_out {
            Some(conn) => {
                conn.send(&buffer)?;
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
        self.connection_out = None;
        self.connection_in = None;
    }

    pub fn midir_test() {
        // --- 1. MIDI Input Setup (for receiving) ---
        let midi_in = MidiInput::new("Rust MIDI Input Client").unwrap();
        let in_ports = midi_in.ports();
        let mut in_port_idx = None;

        if in_ports.is_empty() {
            println!("No MIDI input devices found.");
        } else {
            for (i, p) in in_ports.iter().enumerate() {
                if midi_in.port_name(p).unwrap().starts_with("NUX MG-30") {
                    in_port_idx = Some(i);
                    break;
                }
            }
        }
        let in_port = in_ports.get(in_port_idx.unwrap()).unwrap();

        let conn_in: MidiInputConnection<()> = midi_in
            .connect(
                in_port,
                "midir-read-sysex",
                move |stamp, message, _| {
                    // Callback pour les messages entrants
                    let _ = stamp; // not used in this case

                    if !message.is_empty() && message[0] == 0xf0 {
                        // check if SysEx
                        // SysEx messages are starting with 0xf0 and ends with 0xf7
                        println!(
                            " Raw MIDI SysEx message ({} bytes): {:?}",
                            message.len(),
                            message
                        );

                        if message.len() >= 3
                            && message[0] == 0xF0
                            && message[message.len() - 1] == 0xF7
                        {
                            let manufacturer_id = message[1];
                            let data = &message[2..message.len() - 1]; // Exclut F0, F7 et l'ID fabricant (simple cas)
                            println!("  Manufacturer ID (simplified) : 0x{:X}", manufacturer_id);
                            println!("  Useful data (simplified) : {:?}", data);
                        }
                    }
                },
                (), // User data for the callback
            )
            .unwrap();

        // --- 2. MIDI Output Setup (for sending) ---
        let midi_out = MidiOutput::new("Rust MIDI Output Client").unwrap();
        let out_ports = midi_out.ports();
        let mut out_conn: Option<MidiOutputConnection> = None;

        if out_ports.is_empty() {
            println!("No MIDI output devices found.");
        } else {
            for p in out_ports.iter() {
                if midi_out.port_name(p).unwrap().starts_with("NUX MG-30") {
                    let port_name = midi_out.port_name(p).unwrap();
                    out_conn = Some(midi_out.connect(p, &port_name).unwrap());
                    break;
                }
            }
        }

        // --- 3. Send MIDI Messages (if output port is open) ---
        if let Some(mut conn) = out_conn {
            println!("\nSending example MIDI messages...");

            // let note_on_message = &crate::nux_sys_ex_commands::DEVICE_VERSION_REQUEST;
            println!(
                "[MIDI OUT] Version request: {:?}",
                &nux_audio::nux_sys_ex_commands::DEVICE_VERSION_REQUEST
            );
            conn.send(&[0xf0, 0x43, 0x58, 0x00, 0xf7]).unwrap();

            // Send a MIDI System Exclusive (SysEx) Identity Request
            // F0 7E 7F 06 01 F7 (Universal SysEx, Non-realtime, Identity Request)
            let sysex_identity_request = &[0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7];
            println!(
                "[MIDI OUT] Sending SysEx Identity Request: {:?}",
                sysex_identity_request
            );
            conn.send(sysex_identity_request).unwrap();

            println!("Finished sending messages. Keeping output port open...");
        } else {
            println!("No output port selected or opened. Skipping sending messages.");
        }

        // --- Keep the application running to receive messages ---
        println!("\nApplication running. Press Enter to exit.");
        let _ = stdin().read_line(&mut String::new()); // Wait for user to press Enter
    }
}

impl Drop for MidiManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}

// --- Corrected Callback function for incoming MIDI messages ---
// The third parameter is `data: &mut D`, where D is the type of the user data passed to `connect`.
// Since we passed `()`, it will be `data: &mut ()`.
fn midi_input_callback(timestamp: u64, message: &[u8], _data: &mut ()) {
    println!(
        "[MIDI IN] Timestamp: {} - Message: {:?}",
        timestamp, message
    );
    if !message.is_empty() {
        match message[0] {
            0xF0 => {
                // System Exclusive (SysEx) message
                println!("  SysEx message received!");
            }
            0x90..=0x9F => println!("  Note On message received! Channel: {}", message[0] & 0x0F),
            0x80..=0x8F => println!(
                "  Note Off message received! Channel: {}",
                message[0] & 0x0F
            ),
            _ => {}
        }
    }
}

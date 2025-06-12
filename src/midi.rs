use anyhow::Result; // For ergonomic error handling
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort, MidiOutput,
    MidiOutputConnection, MidiOutputPort, PortInfoError,
};
use std::io::{stdin, stdout, Write};

pub fn print_inputs() {
    println!("MIDI Input Ports:");
    match MidiInput::new("midir test input") {
        Ok(midi_in) => {
            let in_ports = midi_in.ports();

            if in_ports.is_empty() {
                println!("  No MIDI input ports found.");
            } else {
                for (i, p) in in_ports.iter().enumerate() {
                    print_port_name(i, midi_in.port_name(p));
                }
            }
        }
        Err(e) => eprintln!("Cannot get MIDI ins: {e}"),
    }
}

pub fn print_outputs() {
    println!("\nMIDI Output Ports:");
    match MidiOutput::new("midir test output") {
        Ok(midi_out) => {
            let out_ports = midi_out.ports();

            if out_ports.is_empty() {
                println!("  No MIDI output ports found.");
            } else {
                for (i, p) in out_ports.iter().enumerate() {
                    print_port_name(i, midi_out.port_name(p));
                }
            }
        }
        Err(e) => eprintln!("Cannot get MIDI outs: {e}"),
    }
}

fn print_port_name(index: usize, result: Result<String, PortInfoError>) {
    match result {
        Ok(port_name) => println!("  [{}] {}", index, port_name),
        Err(e) => eprintln!("Cannot get port_name: {e}"),
    }
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

    let conn_in: MidiInputConnection<()> = midi_in.connect(
        in_port,
        "midir-read-sysex",
        move |stamp, message, _| {
            // Callback pour les messages entrants
            let _ = stamp; // not used in this case

            if !message.is_empty() && message[0] == 0xf0 { // check if SysEx
                // SysEx messages are starting with 0xf0 and ends with 0xf7
                println!(" Raw MIDI SysEx message ({} bytes): {:?}", message.len(), message);

                if message.len() >= 3 && message[0] == 0xF0 && message[message.len() - 1] == 0xF7 {
                    let manufacturer_id = message[1];
                    let data = &message[2..message.len() - 1]; // Exclut F0, F7 et l'ID fabricant (simple cas)
                    println!("  Manufacturer ID (simplified) : 0x{:X}", manufacturer_id);
                    println!("  Useful data (simplified) : {:?}", data);
                }
            }
        },
        (), // User data for the callback
    ).unwrap();

    // --- 2. MIDI Output Setup (for sending) ---
    let midi_out = MidiOutput::new("Rust MIDI Output Client").unwrap();
    let out_ports = midi_out.ports();
    let mut out_conn: Option<MidiOutputConnection> = None;

    if out_ports.is_empty() {
        println!("No MIDI output devices found.");
    } else {
        for (_i, p) in out_ports.iter().enumerate() {
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
        println!("[MIDI OUT] Version request: {:?}", &crate::nux_sys_ex_commands::DEVICE_VERSION_REQUEST);
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

// --- Helper to get user input ---
fn get_user_input(prompt: &str) -> String {
    print!("{}", prompt);
    stdout().flush().unwrap();
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
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

//! MIDI/SysEx listener for reverse engineering.
//!
//! Connects to the device input port and prints every received message until
//! the user presses Ctrl+C.  Known messages are printed in green with a
//! human-readable label; unknown messages are printed as raw hex only.
//! A `---` separator is printed between messages.

use midir::MidiInput;
use tokio::sync::mpsc;

// ── ANSI colour helpers ───────────────────────────────────────────────────────

const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

// ── Known CC numbers ──────────────────────────────────────────────────────────

/// Maps a CC number to a human-readable label when we know what it means.
fn describe_cc(cc: u8, value: u8) -> Option<String> {
    // Bypass toggles (CC 0-10)
    let bypass_slot = match cc {
        0 => Some("Wah"),
        1 => Some("Comp"),
        2 => Some("Efx"),
        3 => Some("Amp"),
        4 => Some("Eq"),
        5 => Some("Gate"),
        6 => Some("Mod"),
        7 => Some("Delay"),
        8 => Some("Reverb"),
        9 => Some("IR"),
        10 => Some("Send/Return"),
        _ => None,
    };
    if let Some(slot) = bypass_slot {
        let state = if value >= 0x40 { "enable" } else { "bypass" };
        return Some(format!("CC {slot} {state}  (val={value:#04X})"));
    }

    // Knob assignments from MIDI_CC_assignements.md
    let knob_label = match cc {
        0x0B => Some("Wah knob 1"),
        0x0C => Some("Wah knob 2"),
        0x0D => Some("Comp knob 1"),
        0x0E => Some("Comp knob 2"),
        0x0F => Some("Comp knob 3"),
        0x10 => Some("Comp knob 4"),
        0x11 => Some("Efx knob 1"),
        0x12 => Some("Efx knob 2"),
        0x13 => Some("Efx knob 3"),
        0x14 => Some("Efx knob 4"),
        0x15 => Some("Efx knob 5"),
        0x16 => Some("Amp knob 1"),
        0x17 => Some("Amp knob 2"),
        0x18 => Some("Amp knob 3"),
        0x19 => Some("Amp knob 4"),
        0x1A => Some("Amp knob 5"),
        0x1B => Some("Amp knob 6"),
        0x1C => Some("Amp knob 7"),
        0x1D => Some("Amp knob 8"),
        0x1E => Some("EQ knob 1"),
        0x1F => Some("EQ knob 2"),
        0x20 => Some("EQ knob 3"),
        0x21 => Some("EQ knob 4"),
        0x22 => Some("EQ knob 5"),
        0x23 => Some("EQ knob 6"),
        0x24 => Some("EQ knob 7"),
        0x25 => Some("EQ knob 8"),
        0x26 => Some("EQ knob 9"),
        0x27 => Some("EQ knob 10"),
        0x28 => Some("EQ knob 11"),
        0x29 => Some("EQ knob 12"),
        0x2A => Some("Gate knob 1"),
        0x2B => Some("Gate knob 2"),
        0x2C => Some("Gate knob 3"),
        0x2D => Some("Gate knob 4"),
        0x2E => Some("Mod knob 1"),
        0x2F => Some("Mod knob 2"),
        0x30 => Some("Mod knob 3"),
        0x31 => Some("Mod knob 4"),
        0x32 => Some("Mod knob 5"),
        0x33 => Some("Mod knob 6"),
        0x34 => Some("Delay knob 1"),
        0x35 => Some("Delay knob 2"),
        0x36 => Some("Delay knob 3"),
        0x37 => Some("Delay knob 4"),
        0x38 => Some("Reverb knob 1"),
        0x39 => Some("Reverb knob 2"),
        0x3A => Some("Reverb knob 3"),
        0x3B => Some("Reverb knob 4"),
        0x3C => Some("IR knob 1"),
        0x3D => Some("IR knob 2"),
        0x5A => Some("Comp knob 5"),
        // Scene change
        0x4F => {
            return Some(format!("Scene change → {}", scene_label(value)));
        }
        _ => None,
    };
    knob_label.map(|label| format!("CC {label}  val={value} ({value:#04X})"))
}

fn scene_label(value: u8) -> &'static str {
    match value {
        0 => "A",
        1 => "B",
        2 => "C",
        _ => "?",
    }
}

// ── Known SysEx patterns ──────────────────────────────────────────────────────

fn describe_sysex(msg: &[u8]) -> Option<String> {
    // All known Nux SysEx starts with F0 43 58
    if msg.len() < 4 || msg[0] != 0xF0 || msg[1] != 0x43 || msg[2] != 0x58 {
        return None;
    }

    let label = match &msg[3..] {
        // Requests (short, sent to device — we may receive our own output on loopback)
        [0x00, 0xF7] => "SysEx  Device version request",
        [0x70, 0x15, 0x00, 0xF7] => "SysEx  Current preset basic request",
        [0x70, 0x0C, 0x00, 0xF7] => "SysEx  Effect order request",

        // Responses (longer, sent from device)
        [0x10, ..] => "SysEx  Device version response",
        [0x70, 0x15, 0x02, ..] => "SysEx  Current preset basic response",
        [0x70, 0x0B, 0x02, ..] => "SysEx  Current preset detail response",
        [0x70, 0x0C, 0x02, ..] => "SysEx  Effect order response",
        [0x70, 0x7E, 0x02, 0x0D, ..] => "SysEx  Effect order changed notification",

        // MG-30 preset request  F0 43 58 70 0B [slot] F7
        [0x70, 0x0B, slot, 0xF7] => {
            return Some(format!("SysEx  Request preset detail  slot={slot:#04X}"));
        }

        _ => return None,
    };
    Some(format!("{label}  ({} bytes)", msg.len()))
}

// ── Message printer ───────────────────────────────────────────────────────────

fn hex_line(msg: &[u8]) -> String {
    msg.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn print_message(msg: &[u8]) {
    match msg.first() {
        // Control Change: B0 cc value
        Some(0xB0) if msg.len() == 3 => {
            let cc = msg[1];
            let value = msg[2];
            if let Some(desc) = describe_cc(cc, value) {
                println!("{GREEN}{desc}{RESET}");
            }
            println!("  {}", hex_line(msg));
        }

        // Program Change: C0 preset
        Some(0xC0) if msg.len() >= 2 => {
            let preset = msg[1];
            println!("{GREEN}Preset change  preset={preset} ({preset:#04X}){RESET}");
            println!("  {}", hex_line(msg));
        }

        // SysEx: F0 ... F7
        Some(0xF0) => {
            if let Some(desc) = describe_sysex(msg) {
                println!("{GREEN}{desc}{RESET}");
            }
            println!("  {}", hex_line(msg));
        }

        // Note On / Off, etc. — just raw hex
        _ => {
            println!("  {}", hex_line(msg));
        }
    }
    println!("---");
}

// ── Command entry point ───────────────────────────────────────────────────────

pub async fn run(device_name: &Option<String>) {
    let (device, port_in, _port_out) = match super::resolve_ports(device_name) {
        Some(p) => p,
        None => return,
    };

    println!("Listening on {} — press Ctrl+C to stop", device.model);
    println!("---");

    let input = match MidiInput::new("nux-listen") {
        Ok(i) => i,
        Err(e) => {
            eprintln!("MIDI init error: {e}");
            return;
        }
    };

    let ports = input.ports();
    let port = match ports.get(port_in) {
        Some(p) => p,
        None => {
            eprintln!("MIDI input port {port_in} not found");
            return;
        }
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let _conn = match input.connect(
        port,
        "nux-listen",
        move |_stamp, msg, _| {
            let _ = tx.send(msg.to_vec());
        },
        (),
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot open MIDI input: {e}");
            return;
        }
    };

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("Stopped.");
                break;
            }
            Some(msg) = rx.recv() => {
                print_message(&msg);
            }
        }
    }
}

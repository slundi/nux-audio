// CLI entry point: defines `Cli`, `Commands`, the command router, and shared helpers.

mod command_bypass;
mod command_devices;
mod command_effect;
mod command_info;
mod command_model;
mod command_preset;
pub mod error;

use crate::device::Device;
use crate::export::OutputFormat;
use clap::{Parser, Subcommand};
use command_bypass::parse_bypass_state;
use command_devices::list as list_devices;
use command_effect::parse_effect_parameters;
use nux_audio::string::{decimal_string_to_vec_u8, hex_string_to_vec_u8};

// Re-export types that appear in the public Commands enum or tests
pub use command_bypass::BypassState;
pub use command_effect::{EffectParameter, KnobIdentifier, parse_effect_params_from_string};
pub use command_preset::{GetPresetArgs, PresetAction, SendPresetArgs};

#[derive(Parser)]
#[command(name = "nux-audio")]
#[command(version = "0.1.0")]
#[command(about = "Guitar multi-effects manager via MIDI")]
#[command(long_about = None)]
pub struct Cli {
    /// Device name to use
    #[arg(short = 'd', long = "device", global = true)]
    pub device_name: Option<String>,

    /// MIDI port to use (alternative to device name)
    #[arg(short = 'p', long = "port", global = true)]
    pub midi_port: Option<usize>,

    /// Output preset data as YAML (default)
    #[arg(long, global = true, conflicts_with = "json")]
    pub yaml: bool,

    /// Output preset data as JSON
    #[arg(long, global = true, conflicts_with = "yaml")]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Resolve the output format from `--json` / `--yaml` flags. Defaults to YAML.
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::Yaml
        }
    }

    pub async fn process(self) {
        let format = self.output_format();
        match self.command {
            Commands::Devices => {
                println!("Listing available devices...");
                let ports = list_devices();
                println!("{} MIDI ports found", ports.len());
                for port in ports {
                    // TODO: highlight Nux devices
                    println!(
                        "* {}\t{}\t{}",
                        port.port,
                        if port.is_output { "OUT" } else { "IN" },
                        port.name
                    );
                }
            }
            Commands::Info => command_info::run(&self.device_name).await,
            Commands::Preset { action } => command_preset::run(&self.device_name, action, format).await,
            Commands::Effect {
                ref effect,
                ref parameters,
            } => command_effect::run(&self.device_name, effect, parameters).await,
            Commands::Bypass {
                ref category,
                state,
            } => command_bypass::run(&self.device_name, category, state).await,
            Commands::Model {
                ref category,
                ref model,
            } => command_model::run(&self.device_name, category, model).await,
            Commands::Raw {
                ref data,
                decimal,
                with_prefixes: _,
            } => {
                // TODO: prepare string to be sent
                let prepared = data.join(" ");
                if decimal {
                    match decimal_string_to_vec_u8(&prepared) {
                        Ok(cleaned) => send_raw(&cleaned),
                        Err(e) => eprintln!("{e}"),
                    }
                } else {
                    match hex_string_to_vec_u8(&prepared) {
                        Ok(cleaned) => send_raw(&cleaned),
                        Err(e) => eprintln!("{e}"),
                    }
                }
            }
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available MIDI devices
    Devices,

    /// Display connected device information
    Info,

    /// Preset management
    Preset {
        #[command(subcommand)]
        action: PresetAction,
    },

    /// Effect control — set knob values via MIDI CC
    ///
    /// Example: nux effect amp gain=80 tone=50
    ///          nux effect delay 52=64
    Effect {
        /// Effect slot name (wah, amp, reverb, …)
        effect: String,

        /// Knob settings in "knob=value" format
        /// Example: "gain=80,tone=50" or "22=80"
        #[arg(value_parser = parse_effect_parameters)]
        parameters: Vec<EffectParameter>,
    },

    /// Switch the active model within an effect slot
    ///
    /// The model can be a hex ID (0x45), decimal ID (69), or model name ("Tweed 1x8").
    ///
    /// Example: nux model amp 0x45
    ///          nux model reverb "Hall"
    Model {
        /// Effect slot name (wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir)
        category: String,

        /// Model to activate: hex ID (0x45), decimal ID (69), or model name ("Hall")
        model: String,
    },

    /// Toggle the bypass state of an effect slot
    ///
    /// Example: nux bypass amp on
    ///          nux bypass reverb off
    Bypass {
        /// Effect slot name (wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir)
        category: String,

        /// Desired state: "on" to activate, "off" to bypass
        #[arg(value_parser = parse_bypass_state)]
        state: BypassState,
    },

    /// Send raw SysEx data (start and end bytes are added automatically)
    Raw {
        /// Hex or decimal byte sequence
        data: Vec<String>,

        /// Parse data as decimal instead of hex
        #[arg(long)]
        decimal: bool,

        /// Include custom device-ID prefix bytes
        #[arg(long)]
        with_prefixes: bool,
    },
}

/// Detect the target device and resolve its (port_in, port_out) MIDI port indices.
///
/// Honours `--device` / `--port` selection from the CLI.  Prints a user-facing
/// error and returns `None` when no suitable device can be found.
fn resolve_ports(device_name: &Option<String>) -> Option<(Device, usize, usize)> {
    let mut devices = Device::detect();

    let device = if let Some(name) = device_name {
        let n = name.to_lowercase();
        match devices
            .iter()
            .position(|d| d.name.to_lowercase().contains(&n) || d.model.to_lowercase() == n)
        {
            Some(i) => devices.swap_remove(i),
            None => {
                eprintln!("Device '{name}' not found. Use `nux devices` to list ports.");
                return None;
            }
        }
    } else {
        match devices.len() {
            0 => {
                eprintln!("No supported Nux device detected. Is the device connected?");
                return None;
            }
            1 => devices.swap_remove(0),
            _ => {
                eprintln!(
                    "Multiple devices found — specify one with -d <model>: {}",
                    devices
                        .iter()
                        .map(|d| d.model.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return None;
            }
        }
    };

    let port_in = match device.inputs.values().copied().next() {
        Some(p) => p,
        None => {
            eprintln!("No MIDI input port for {}", device.model);
            return None;
        }
    };
    let port_out = match device.outputs.values().copied().next() {
        Some(p) => p,
        None => {
            eprintln!("No MIDI output port for {}", device.model);
            return None;
        }
    };

    Some((device, port_in, port_out))
}

fn send_raw(data: &[u8]) {
    use midir::MidiOutput;
    let midi_out = match MidiOutput::new("nux-raw") {
        Ok(o) => o,
        Err(e) => { eprintln!("MIDI init error: {e}"); return; }
    };
    let ports = midi_out.ports();
    let port = match ports.iter().find(|p| {
        midi_out.port_name(p).map(|n| n.to_lowercase().contains("nux")).unwrap_or(false)
    }) {
        Some(p) => p,
        None => { eprintln!("No NUX device found"); return; }
    };
    let mut conn = match midi_out.connect(port, "nux-raw-out") {
        Ok(c) => c,
        Err(e) => { eprintln!("MIDI connect error: {e}"); return; }
    };
    let mut msg = vec![0xF0, 0x43];
    msg.extend_from_slice(data);
    msg.push(0xF7);
    match conn.send(&msg) {
        Ok(()) => println!("Sent {} bytes: {}", msg.len(), msg.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")),
        Err(e) => eprintln!("Send error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_devices_command() {
        let args = ["guitar-fx", "devices"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(matches!(cli.command, Commands::Devices));
    }

    #[test]
    fn test_info_with_device() {
        let args = ["guitar-fx", "-d", "MG30", "info"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.device_name, Some("MG30".to_string()));
        assert!(matches!(cli.command, Commands::Info));
    }

    #[test]
    fn test_preset_get() {
        let args = ["guitar-fx", "preset", "get", "-s", "3", "my_preset.json"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Preset {
            action: PresetAction::Get(get_args),
        } = cli.command
        {
            assert_eq!(get_args.slot, Some(3));
            assert_eq!(get_args.filename, Some(PathBuf::from("my_preset.json")));
        } else {
            panic!("Expected preset get command");
        }
    }

    #[test]
    fn test_preset_send() {
        let args = ["guitar-fx", "preset", "send", "preset.json"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Preset {
            action: PresetAction::Send(send_args),
        } = cli.command
        {
            assert_eq!(send_args.slot, None);
            assert_eq!(send_args.filename, PathBuf::from("preset.json"));
        } else {
            panic!("Expected preset send command");
        }
    }

    #[test]
    fn test_effect_with_numeric_params() {
        let args = ["guitar-fx", "effect", "distortion", "1=80", "2=50"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Effect { effect, parameters } = cli.command {
            assert_eq!(effect, "distortion");
            assert_eq!(parameters.len(), 2);
            match &parameters[0].knob {
                KnobIdentifier::Id(1) => assert_eq!(parameters[0].value, 80),
                _ => panic!("Expected knob ID 1"),
            }
        } else {
            panic!("Expected effect command");
        }
    }

    #[test]
    fn test_effect_with_named_params() {
        let args = ["guitar-fx", "effect", "reverb", "gain=75", "tone=60"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Effect { effect, parameters } = cli.command {
            assert_eq!(effect, "reverb");
            assert_eq!(parameters.len(), 2);
            match &parameters[0].knob {
                KnobIdentifier::Name(name) if name == "gain" => {
                    assert_eq!(parameters[0].value, 75)
                }
                _ => panic!("Expected knob name 'gain'"),
            }
        } else {
            panic!("Expected effect command");
        }
    }

    #[test]
    fn test_model_command_hex_id() {
        let args = ["guitar-fx", "model", "amp", "0x45"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Model { category, model } = cli.command {
            assert_eq!(category, "amp");
            assert_eq!(model, "0x45");
        } else {
            panic!("Expected model command");
        }
    }

    #[test]
    fn test_model_command_name() {
        let args = ["guitar-fx", "model", "reverb", "Hall"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Model { category, model } = cli.command {
            assert_eq!(category, "reverb");
            assert_eq!(model, "Hall");
        } else {
            panic!("Expected model command");
        }
    }

    #[test]
    fn test_model_command_decimal_id() {
        let args = ["guitar-fx", "model", "delay", "69"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Model { category, model } = cli.command {
            assert_eq!(category, "delay");
            assert_eq!(model, "69");
        } else {
            panic!("Expected model command");
        }
    }

    #[test]
    fn test_bypass_on_command() {
        let args = ["guitar-fx", "bypass", "amp", "on"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Bypass { category, state } = cli.command {
            assert_eq!(category, "amp");
            assert_eq!(state, BypassState::On);
        } else {
            panic!("Expected bypass command");
        }
    }

    #[test]
    fn test_bypass_off_command() {
        let args = ["guitar-fx", "bypass", "reverb", "off"];
        let cli = Cli::try_parse_from(args).unwrap();
        if let Commands::Bypass { category, state } = cli.command {
            assert_eq!(category, "reverb");
            assert_eq!(state, BypassState::Off);
        } else {
            panic!("Expected bypass command");
        }
    }
}

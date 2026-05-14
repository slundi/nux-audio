mod command_devices;
pub mod error;

use crate::export::OutputFormat;
use crate::midi::MidiManager;
use clap::{Args, Parser, Subcommand};
use command_devices::list as list_devices;
use std::path::PathBuf;

use nux_audio::{
    nux_sys_ex_commands::DEVICE_VERSION_REQUEST,
    string::{decimal_string_to_vec_u8, hex_string_to_vec_u8},
};

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
    /// Resolve the output format from `--json` / `--yaml` flags.
    /// Defaults to YAML when neither flag is set.
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::Yaml
        }
    }
}

impl Cli {
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
            Commands::Info => self.info().await,
            Commands::Preset { action } => match action {
                PresetAction::Show => {
                    // TODO: fetch current preset from device, then:
                    // let file = PresetFile::from_preset(&preset, &device.model);
                    // println!("{}", file.to_string_fmt(format).unwrap());
                    println!("Showing current preset... (format: {format:?})");
                }
                PresetAction::Get(args) => {
                    // TODO: fetch preset slot from device, then:
                    // let file = PresetFile::from_preset(&preset, &device.model);
                    // match args.filename {
                    //     Some(path) => file.write_to_file(&path, format).unwrap(),
                    //     None => println!("{}", file.to_string_fmt(format).unwrap()),
                    // }
                    println!(
                        "Getting preset from slot: {:?}, saving to: {:?} (format: {format:?})",
                        args.slot, args.filename
                    );
                }
                PresetAction::Send(args) => {
                    // TODO: PresetFile::read_from_file(&args.filename), then send SysEx
                    println!(
                        "Sending preset from: {:?} to slot: {:?}",
                        args.filename, args.slot
                    );
                }
            },
            Commands::Effect {
                ref effect,
                ref parameters,
            } => {
                self.send_effect_cc(effect, parameters).await;
            }
            Commands::Bypass {
                ref category,
                state,
            } => {
                self.send_bypass_cc(category, state).await;
            }
            Commands::Model {
                ref category,
                ref model,
            } => {
                self.send_model_change(category, model).await;
            }
            Commands::Raw {
                ref data,
                decimal,
                with_prefixes: _,
            } => {
                // TODO: prepare string to be sent
                let prepared = data.join(" ");
                if decimal {
                    match decimal_string_to_vec_u8(&prepared) {
                        Ok(cleaned) => self.send_raw(&cleaned),
                        Err(e) => eprintln!("{e}"),
                    }
                } else {
                    match hex_string_to_vec_u8(&prepared) {
                        Ok(cleaned) => self.send_raw(&cleaned),
                        Err(e) => eprintln!("{e}"),
                    }
                }
            }
        };
    }

    fn send_raw(self, data: &[u8]) {
        println!("Sending message: {:?} (replace with formated string", data);
        // TODO: get device
    }

    /// Detect the target device and return its (port_in, port_out) indices.
    ///
    /// Prints an error and returns `None` when no suitable device is found.
    fn resolve_ports(&self) -> Option<(crate::device::Device, usize, usize)> {
        use crate::device::Device;

        let mut devices = Device::detect();

        let device = if let Some(name) = &self.device_name {
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

    /// Send MIDI CC messages for knob parameters on the given effect category.
    ///
    /// Named knobs (e.g. "gain") are resolved to CC numbers via the static catalog.
    /// Numeric knobs are treated as CC numbers directly.
    async fn send_effect_cc(&self, effect: &str, parameters: &[EffectParameter]) {
        use crate::effects::{EffectCategory, find_cc_for_knob};

        let category = match effect.parse::<EffectCategory>() {
            Ok(c) => c,
            Err(()) => {
                eprintln!(
                    "Unknown effect category '{effect}'. \
                     Try: wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir, vol"
                );
                return;
            }
        };

        let (device, _port_in, port_out) = match self.resolve_ports() {
            Some(t) => t,
            None => return,
        };

        let mut manager = match MidiManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Cannot create MIDI manager: {e}");
                return;
            }
        };

        if let Err(e) = manager.connect_output(port_out) {
            eprintln!("Cannot connect to {}: {e}", device.model);
            return;
        }

        let mut errors = 0usize;
        for param in parameters {
            let cc = match &param.knob {
                KnobIdentifier::Id(n) => *n,
                KnobIdentifier::Name(name) => match find_cc_for_knob(category, name.as_str()) {
                    Some(cc) => cc,
                    None => {
                        eprintln!(
                            "Knob '{name}' not found in {} catalog. \
                             Use a CC number directly (e.g. 22=80).",
                            category.as_str()
                        );
                        errors += 1;
                        continue;
                    }
                },
            };
            match manager.send_cc(cc, param.value) {
                Ok(()) => println!(
                    "CC{cc}={} → {} ({})",
                    param.value,
                    category.as_str(),
                    device.model
                ),
                Err(e) => {
                    eprintln!("Failed to send CC{cc}={}: {e}", param.value);
                    errors += 1;
                }
            }
        }

        if errors > 0 {
            eprintln!("{errors} error(s) occurred");
        }
    }

    /// Switch the active model within an effect slot via SysEx.
    ///
    /// `model_str` accepts: `"0x45"` (hex), `"69"` (decimal), or a model name
    /// matched case-insensitively against the built-in catalog.
    ///
    /// The SysEx format is provisional pending hardware verification.
    async fn send_model_change(&self, category_str: &str, model_str: &str) {
        use crate::effects::{EffectCategory, find_model_by_name};
        use nux_audio::nux_sys_ex_commands::mg30_change_effect_model;

        let category = match category_str.parse::<EffectCategory>() {
            Ok(c) => c,
            Err(()) => {
                eprintln!(
                    "Unknown effect category '{category_str}'. \
                     Try: wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir"
                );
                return;
            }
        };

        // Resolve model: hex (0x…), decimal, or name search
        let (model_id, model_title) = if let Some(hex) = model_str
            .strip_prefix("0x")
            .or_else(|| model_str.strip_prefix("0X"))
        {
            match u8::from_str_radix(hex, 16) {
                Ok(id) => (id, None),
                Err(_) => {
                    eprintln!("Invalid hex model ID '{model_str}'");
                    return;
                }
            }
        } else if let Ok(id) = model_str.parse::<u8>() {
            (id, None)
        } else {
            match find_model_by_name(category, model_str) {
                Some((id, opt)) => (id, Some(opt.title)),
                None => {
                    eprintln!(
                        "Model '{model_str}' not found in {} catalog",
                        category.as_str()
                    );
                    return;
                }
            }
        };

        let (device, _port_in, port_out) = match self.resolve_ports() {
            Some(t) => t,
            None => return,
        };

        let mut manager = match MidiManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Cannot create MIDI manager: {e}");
                return;
            }
        };

        if let Err(e) = manager.connect_output(port_out) {
            eprintln!("Cannot connect to {}: {e}", device.model);
            return;
        }

        let payload = mg30_change_effect_model(category as u8, model_id);
        match manager.send_sysex(&payload) {
            Ok(()) => {
                let name_part = model_title.map(|t| format!(" ({t})")).unwrap_or_default();
                println!(
                    "{} model → 0x{model_id:02X}{name_part} [provisional SysEx] ({})",
                    category.as_str(),
                    device.model
                );
            }
            Err(e) => eprintln!("Failed to send model change SysEx: {e}"),
        }
    }

    /// Send the bypass CC for an effect slot.
    ///
    /// `state = On` → CC value `0x7F` (slot active)
    /// `state = Off` → CC value `0x00` (slot bypassed)
    async fn send_bypass_cc(&self, category_str: &str, state: BypassState) {
        use crate::effects::{CC_BYPASS_OFF, CC_BYPASS_ON, EffectCategory};

        let category = match category_str.parse::<EffectCategory>() {
            Ok(c) => c,
            Err(()) => {
                eprintln!(
                    "Unknown effect category '{category_str}'. \
                     Try: wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir"
                );
                return;
            }
        };

        let cc = match category.bypass_cc() {
            Some(cc) => cc,
            None => {
                eprintln!("The '{}' slot has no bypass toggle.", category.as_str());
                return;
            }
        };

        let (device, _port_in, port_out) = match self.resolve_ports() {
            Some(t) => t,
            None => return,
        };

        let mut manager = match MidiManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Cannot create MIDI manager: {e}");
                return;
            }
        };

        if let Err(e) = manager.connect_output(port_out) {
            eprintln!("Cannot connect to {}: {e}", device.model);
            return;
        }

        let value = match state {
            BypassState::On => CC_BYPASS_ON,
            BypassState::Off => CC_BYPASS_OFF,
        };

        match manager.send_cc(cc, value) {
            Ok(()) => println!(
                "{} {} → CC{cc}=0x{value:02X} ({})",
                category.as_str(),
                if state == BypassState::On {
                    "ON"
                } else {
                    "OFF"
                },
                device.model
            ),
            Err(e) => eprintln!("Failed to send bypass CC: {e}"),
        }
    }

    async fn info(self) {
        use crate::device::{Device, extract_version};
        use crate::registry::REGISTRY;
        use std::time::Duration;

        const VERSION_HEADER: [u8; 4] = [0xF0, 0x43, 0x58, 0x10];
        const TIMEOUT: Duration = Duration::from_secs(3);

        // Detect supported devices on the system
        let mut devices = Device::detect();

        // Select target device
        let device = if let Some(name) = &self.device_name {
            let n = name.to_lowercase();
            match devices
                .iter()
                .position(|d| d.name.to_lowercase().contains(&n) || d.model.to_lowercase() == n)
            {
                Some(i) => devices.swap_remove(i),
                None => {
                    eprintln!("Device '{name}' not found. Use `nux devices` to list ports.");
                    return;
                }
            }
        } else {
            match devices.len() {
                0 => {
                    eprintln!("No supported Nux device detected. Is the device connected?");
                    return;
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
                    return;
                }
            }
        };

        // Resolve port indices (both input and output are required)
        let port_in = match device.inputs.values().copied().next() {
            Some(p) => p,
            None => {
                eprintln!("No MIDI input port for {}", device.model);
                return;
            }
        };
        let port_out = match device.outputs.values().copied().next() {
            Some(p) => p,
            None => {
                eprintln!("No MIDI output port for {}", device.model);
                return;
            }
        };

        let mut manager = match MidiManager::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Cannot create MIDI manager: {e}");
                return;
            }
        };

        // Connect via async dispatcher — bridges sync midir callback into async tokio
        let dispatcher = match manager.connect_with_dispatcher(port_in, port_out) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Cannot connect to {}: {e}", device.model);
                return;
            }
        };

        if let Err(e) = manager.send_sysex(&DEVICE_VERSION_REQUEST) {
            eprintln!("Failed to send version request: {e}");
            return;
        }

        // Wait asynchronously for the version response (starts with VERSION_HEADER)
        let response = match dispatcher.await_response(&VERSION_HEADER, TIMEOUT).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

        let firmware = match extract_version(&response) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Could not parse firmware version: {e}");
                return;
            }
        };

        // Print device summary
        println!("Device:       {}", device.name);
        println!("Model:        {}", device.model);
        println!("Device ID:    0x{:02X}", device.device_id);
        println!("Firmware:     {firmware}");
        if let Some(spec) = REGISTRY.by_device_id(device.device_id) {
            println!("Presets:      {}", spec.preset_count);
            println!("Signal chain: {}", spec.signal_chain.join(" → "));
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

    /// Effect control
    Effect {
        /// Effect position or name (e.g., 1, "distortion", "reverb")
        effect: String,

        /// Knob settings in "knob=value" format separated by commas
        /// Example: "gain=80,tone=50" or "1=80,2=50"
        #[arg(value_parser = parse_effect_parameters)]
        parameters: Vec<EffectParameter>,
    },

    /// Switch the active model within an effect slot
    ///
    /// The model can be specified as a hex ID (0x45), a decimal ID (69), or
    /// a model name ("Tweed 1x8").  The name is matched case-insensitively
    /// against the built-in catalog.
    ///
    /// Example: nux model amp 0x45
    ///          nux model reverb "Hall"
    ///          nux model delay 84
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

    /// Send raw SysEx data, you do not need to give the start and end bytes.
    Raw {
        /// Hex string with space or not or decimal string with spaces
        data: Vec<String>,

        /// Decimal format
        decimal: bool,

        /// After the SysEx start byte, you need to specify some bytes like the device ID. By
        /// default it uses the configured for the device but you can try with your own sequence
        /// in the message data.
        with_prefixes: bool,
    },
}

#[derive(Subcommand)]
pub enum PresetAction {
    /// Display current preset details
    #[command(name = "")]
    Show,

    /// Get preset from device
    Get(GetPresetArgs),

    /// Send preset to device
    Send(SendPresetArgs),
}

#[derive(Args)]
pub struct GetPresetArgs {
    /// Preset slot/location to retrieve from
    #[arg(short = 's', long = "slot")]
    pub slot: Option<u8>,

    /// Output file (optional, otherwise display to console)
    pub filename: Option<PathBuf>,
}

#[derive(Args)]
pub struct SendPresetArgs {
    /// Destination preset slot/location
    #[arg(short = 's', long = "slot")]
    pub slot: Option<u8>,

    /// Preset file to send
    pub filename: PathBuf,
}

#[derive(Debug, Clone)]
pub struct EffectParameter {
    pub knob: KnobIdentifier,
    pub value: u8,
}

#[derive(Debug, Clone)]
pub enum KnobIdentifier {
    Id(u8),
    Name(String),
}

impl std::fmt::Display for EffectParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.knob {
            KnobIdentifier::Id(id) => write!(f, "{}={}", id, self.value),
            KnobIdentifier::Name(name) => write!(f, "{}={}", name, self.value),
        }
    }
}

/// Parse effect parameters in "knob=value,knob2=value2" format
fn parse_effect_parameters(s: &str) -> Result<EffectParameter, String> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid format '{}'. Expected: 'knob=value'", s));
    }

    let knob_str = parts[0].trim();
    let value_str = parts[1].trim();

    // Try to parse as numeric ID first
    let knob = if let Ok(id) = knob_str.parse::<u8>() {
        KnobIdentifier::Id(id)
    } else {
        KnobIdentifier::Name(knob_str.to_string())
    };

    let value = value_str.parse::<u8>().map_err(|_| {
        format!(
            "Invalid value '{}'. Expected: number between 0 and 255",
            value_str
        )
    })?;

    Ok(EffectParameter { knob, value })
}

/// Whether to activate or bypass an effect slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassState {
    On,
    Off,
}

fn parse_bypass_state(s: &str) -> Result<BypassState, String> {
    match s.to_lowercase().as_str() {
        "on" | "1" | "true" | "enable" => Ok(BypassState::On),
        "off" | "0" | "false" | "disable" => Ok(BypassState::Off),
        _ => Err(format!("Expected 'on' or 'off', got '{s}'")),
    }
}

/// Utility function to process effect parameters from command line string
pub fn parse_effect_params_from_string(input: &str) -> Result<Vec<EffectParameter>, String> {
    input
        .split(',')
        .map(|param| parse_effect_parameters(param.trim()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

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
                KnobIdentifier::Name(name) if name == "gain" => assert_eq!(parameters[0].value, 75),
                _ => panic!("Expected knob name 'gain'"),
            }
        } else {
            panic!("Expected effect command");
        }
    }

    #[test]
    fn test_parse_multiple_effect_params() {
        let result = parse_effect_params_from_string("gain=80,tone=50,level=100");
        assert!(result.is_ok());

        let params = result.unwrap();
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].value, 80);
        assert_eq!(params[1].value, 50);
        assert_eq!(params[2].value, 100);
    }

    #[test]
    fn test_invalid_effect_param_format() {
        let result = parse_effect_parameters("invalid_format");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid format"));
    }

    #[test]
    fn test_invalid_effect_param_value() {
        let result = parse_effect_parameters("gain=invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid value"));
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

    #[test]
    fn test_parse_bypass_state_variants() {
        assert_eq!(parse_bypass_state("on"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("ON"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("1"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("enable"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("off"), Ok(BypassState::Off));
        assert_eq!(parse_bypass_state("0"), Ok(BypassState::Off));
        assert_eq!(parse_bypass_state("disable"), Ok(BypassState::Off));
        assert!(parse_bypass_state("maybe").is_err());
    }
}

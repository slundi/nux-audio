// CLI entry point: defines `Cli`, `Commands`, the command router, and shared helpers.
//
// Commands are thin front-ends over `crate::session::NuxSession`: parse arguments,
// call the session, print the result. Errors bubble up as `CliError`.

mod command_bypass;
mod command_devices;
mod command_effect;
mod command_info;
mod command_listen;
mod command_model;
mod command_preset;
pub mod error;

use crate::export::OutputFormat;
use bpaf::*;
use command_bypass::parse_bypass_state;
use command_devices::list as list_devices;
use command_effect::parse_effect_parameters;
use error::CliError;
use nux_audio::effects::EffectCategory;
use nux_audio::string::{decimal_string_to_vec_u8, hex_string_to_vec_u8};
use std::path::PathBuf;

// Re-export types that appear in the public Commands enum or tests
pub use crate::session::{BypassState, EffectParameter, KnobIdentifier};
pub use command_effect::parse_effect_params_from_string;
pub use command_preset::{GetPresetArgs, PresetAction, SendPresetArgs};

/// Parse an effect slot name (`"amp"`, `"reverb"`, …) given on the command line.
fn parse_category(name: &str) -> Result<EffectCategory, CliError> {
    name.parse::<EffectCategory>()
        .map_err(|()| CliError::UnknownCategory(name.to_owned()))
}

#[derive(Clone)]
pub struct Cli {
    pub device_name: Option<String>,
    pub midi_port: Option<usize>,
    format: OutputFormat,
    pub command: Commands,
}

impl Cli {
    pub fn output_format(&self) -> OutputFormat {
        self.format
    }

    /// Run the parsed command. Errors are returned for `main` to report once.
    pub async fn process(self) -> Result<(), CliError> {
        let format = self.output_format();
        let device_name = self.device_name.as_deref();
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
                Ok(())
            }
            Commands::Info => command_info::run(device_name).await,
            Commands::Preset { action } => command_preset::run(device_name, action, format).await,
            Commands::Effect {
                ref effect,
                ref parameters,
            } => command_effect::run(device_name, effect, parameters).await,
            Commands::Bypass {
                ref category,
                state,
            } => command_bypass::run(device_name, category, state).await,
            Commands::Model {
                ref category,
                ref model,
            } => command_model::run(device_name, category, model).await,
            Commands::Listen => command_listen::run(device_name).await,
            Commands::Tui => {
                if let Err(e) = tokio::task::block_in_place(crate::ui::run_tui) {
                    eprintln!("TUI error: {e}");
                }
                Ok(())
            }
            Commands::Raw { ref data, decimal } => {
                eprintln!(
                    "WARNING: Raw SysEx bypasses all safety checks. \
                     Sending incorrect data may cause device malfunctions or data loss."
                );
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
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
pub enum Commands {
    Devices,
    Info,
    Preset {
        action: PresetAction,
    },
    Effect {
        effect: String,
        parameters: Vec<EffectParameter>,
    },
    Bypass {
        category: String,
        state: BypassState,
    },
    Model {
        category: String,
        model: String,
    },
    Listen,
    Tui,
    Raw {
        data: Vec<String>,
        decimal: bool,
    },
}

// ── Parser functions ──────────────────────────────────────────────────────────

fn device_name_parser() -> impl Parser<Option<String>> {
    short('d')
        .long("device")
        .help("Device name to use")
        .argument::<String>("DEVICE")
        .optional()
}

fn midi_port_parser() -> impl Parser<Option<usize>> {
    short('p')
        .long("port")
        .help("MIDI port to use (alternative to device name)")
        .argument::<usize>("PORT")
        .optional()
}

fn output_format_parser() -> impl Parser<OutputFormat> {
    let yaml = long("yaml")
        .help("Output preset data as YAML (default)")
        .req_flag(OutputFormat::Yaml);
    let json = long("json")
        .help("Output preset data as JSON")
        .req_flag(OutputFormat::Json);
    construct!([yaml, json]).fallback(OutputFormat::Yaml)
}

fn cmd_devices() -> impl Parser<Commands> {
    pure(Commands::Devices)
        .to_options()
        .descr("List available MIDI devices")
        .command("devices")
}

fn cmd_info() -> impl Parser<Commands> {
    pure(Commands::Info)
        .to_options()
        .descr("Display connected device information")
        .command("info")
}

fn cmd_listen() -> impl Parser<Commands> {
    pure(Commands::Listen)
        .to_options()
        .descr("Listen for incoming MIDI/SysEx messages (useful for reverse engineering)")
        .command("listen")
}

fn cmd_tui() -> impl Parser<Commands> {
    pure(Commands::Tui)
        .to_options()
        .descr("Launch the interactive TUI")
        .command("tui")
}

fn bypass_state_bpaf() -> impl Parser<BypassState> {
    positional::<String>("STATE")
        .help("Desired state: 'on' to activate, 'off' to bypass")
        .parse(|s| parse_bypass_state(&s))
}

fn cmd_bypass() -> impl Parser<Commands> {
    let category = positional::<String>("CATEGORY")
        .help("Effect slot name (wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir)");
    let state = bypass_state_bpaf();
    construct!(Commands::Bypass { category, state })
        .to_options()
        .descr("Toggle the bypass state of an effect slot")
        .command("bypass")
}

fn effect_parameter_bpaf() -> impl Parser<EffectParameter> {
    positional::<String>("KNOB=VALUE").parse(|s| parse_effect_parameters(&s))
}

fn cmd_effect() -> impl Parser<Commands> {
    let effect = positional::<String>("EFFECT").help("Effect slot name (wah, amp, reverb, …)");
    let parameters = effect_parameter_bpaf().many();
    construct!(Commands::Effect { effect, parameters })
        .to_options()
        .descr("Effect control — set knob values via MIDI CC\n\nExample: nux effect amp gain=80 tone=50")
        .command("effect")
}

fn cmd_model() -> impl Parser<Commands> {
    let category = positional::<String>("CATEGORY")
        .help("Effect slot name (wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir)");
    let model = positional::<String>("MODEL")
        .help("Model to activate: hex ID (0x45), decimal ID (69), or model name (\"Hall\")");
    construct!(Commands::Model { category, model })
        .to_options()
        .descr("Switch the active model within an effect slot")
        .command("model")
}

fn cmd_preset_show() -> impl Parser<PresetAction> {
    pure(PresetAction::Show)
        .to_options()
        .descr("Display current preset details")
        .command("show")
}

fn cmd_preset_get() -> impl Parser<PresetAction> {
    let slot = short('s')
        .long("slot")
        .help("Preset slot/location to retrieve from")
        .argument::<u8>("SLOT")
        .optional();
    let filename = positional::<PathBuf>("FILE")
        .help("Output file (optional, otherwise display to console)")
        .optional();
    construct!(GetPresetArgs { slot, filename })
        .map(PresetAction::Get)
        .to_options()
        .descr("Get preset from device")
        .command("get")
}

fn cmd_preset_send() -> impl Parser<PresetAction> {
    let slot = short('s')
        .long("slot")
        .help("Destination preset slot/location")
        .argument::<u8>("SLOT")
        .optional();
    let filename = positional::<PathBuf>("FILE").help("Preset file to send");
    construct!(SendPresetArgs { slot, filename })
        .map(PresetAction::Send)
        .to_options()
        .descr("Send preset to device")
        .command("send")
}

fn cmd_preset() -> impl Parser<Commands> {
    let action = construct!([cmd_preset_show(), cmd_preset_get(), cmd_preset_send()]);
    construct!(Commands::Preset { action })
        .to_options()
        .descr("Preset management")
        .command("preset")
}

fn cmd_raw() -> impl Parser<Commands> {
    let decimal = long("decimal")
        .help("Parse data as decimal instead of hex")
        .switch();
    // Positional MUST be last in the `construct!` tuple — bpaf panics at
    // parser-build time if a flag follows a positional (see bpaf::meta).
    let data = positional::<String>("DATA")
        .help("Hex or decimal byte sequence")
        .many();
    construct!(Commands::Raw { decimal, data })
        .to_options()
        .descr(
            "Send raw SysEx data (start and end bytes are added automatically)\n\n\
             WARNING: bypasses all safety checks — use only for debugging/reverse-engineering.",
        )
        .command("raw")
}

pub fn cli_parser() -> OptionParser<Cli> {
    let device_name = device_name_parser();
    let midi_port = midi_port_parser();
    let format = output_format_parser();
    let command = construct!([
        cmd_devices(),
        cmd_info(),
        cmd_preset(),
        cmd_effect(),
        cmd_bypass(),
        cmd_model(),
        cmd_listen(),
        cmd_tui(),
        cmd_raw(),
    ]);
    construct!(Cli {
        device_name,
        midi_port,
        format,
        command,
    })
    .to_options()
    .version(env!("CARGO_PKG_VERSION"))
    .descr("Guitar multi-effects manager via MIDI")
}

fn send_raw(data: &[u8]) {
    use midir::MidiOutput;
    let midi_out = match MidiOutput::new("nux-raw") {
        Ok(o) => o,
        Err(e) => {
            eprintln!("MIDI init error: {e}");
            return;
        }
    };
    let ports = midi_out.ports();
    let port = match ports.iter().find(|p| {
        midi_out
            .port_name(p)
            .map(|n| n.to_lowercase().contains("nux"))
            .unwrap_or(false)
    }) {
        Some(p) => p,
        None => {
            eprintln!("No NUX device found");
            return;
        }
    };
    let mut conn = match midi_out.connect(port, "nux-raw-out") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("MIDI connect error: {e}");
            return;
        }
    };
    let mut msg = vec![0xF0, 0x43];
    msg.extend_from_slice(data);
    msg.push(0xF7);
    match conn.send(&msg) {
        Ok(()) => println!(
            "Sent {} bytes: {}",
            msg.len(),
            msg.iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Err(e) => eprintln!("Send error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_devices_command() {
        let cli = cli_parser().run_inner(Args::from(&["devices"])).unwrap();
        assert!(matches!(cli.command, Commands::Devices));
    }

    #[test]
    fn test_info_with_device() {
        let cli = cli_parser()
            .run_inner(Args::from(&["-d", "MG30", "info"]))
            .unwrap();
        assert_eq!(cli.device_name, Some("MG30".to_string()));
        assert!(matches!(cli.command, Commands::Info));
    }

    #[test]
    fn test_preset_get() {
        let cli = cli_parser()
            .run_inner(Args::from(&["preset", "get", "-s", "3", "my_preset.json"]))
            .unwrap();
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
        let cli = cli_parser()
            .run_inner(Args::from(&["preset", "send", "preset.json"]))
            .unwrap();
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
        let cli = cli_parser()
            .run_inner(Args::from(&["effect", "distortion", "1=80", "2=50"]))
            .unwrap();
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
        let cli = cli_parser()
            .run_inner(Args::from(&["effect", "reverb", "gain=75", "tone=60"]))
            .unwrap();
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
        let cli = cli_parser()
            .run_inner(Args::from(&["model", "amp", "0x45"]))
            .unwrap();
        if let Commands::Model { category, model } = cli.command {
            assert_eq!(category, "amp");
            assert_eq!(model, "0x45");
        } else {
            panic!("Expected model command");
        }
    }

    #[test]
    fn test_model_command_name() {
        let cli = cli_parser()
            .run_inner(Args::from(&["model", "reverb", "Hall"]))
            .unwrap();
        if let Commands::Model { category, model } = cli.command {
            assert_eq!(category, "reverb");
            assert_eq!(model, "Hall");
        } else {
            panic!("Expected model command");
        }
    }

    #[test]
    fn test_model_command_decimal_id() {
        let cli = cli_parser()
            .run_inner(Args::from(&["model", "delay", "69"]))
            .unwrap();
        if let Commands::Model { category, model } = cli.command {
            assert_eq!(category, "delay");
            assert_eq!(model, "69");
        } else {
            panic!("Expected model command");
        }
    }

    #[test]
    fn test_bypass_on_command() {
        let cli = cli_parser()
            .run_inner(Args::from(&["bypass", "amp", "on"]))
            .unwrap();
        if let Commands::Bypass { category, state } = cli.command {
            assert_eq!(category, "amp");
            assert_eq!(state, BypassState::On);
        } else {
            panic!("Expected bypass command");
        }
    }

    #[test]
    fn test_listen_command() {
        let cli = cli_parser().run_inner(Args::from(&["listen"])).unwrap();
        assert!(matches!(cli.command, Commands::Listen));
    }

    #[test]
    fn test_bypass_off_command() {
        let cli = cli_parser()
            .run_inner(Args::from(&["bypass", "reverb", "off"]))
            .unwrap();
        if let Commands::Bypass { category, state } = cli.command {
            assert_eq!(category, "reverb");
            assert_eq!(state, BypassState::Off);
        } else {
            panic!("Expected bypass command");
        }
    }

    /// Regression: bpaf panicked at parser-build time when a positional
    /// preceded any flag in `construct!`. Constructing the parser and
    /// asking for --help exercises the same code path that crashed.
    #[test]
    fn test_raw_help_does_not_panic() {
        let parser = cli_parser();
        // `--help` returns `Err(ParseFailure::Stdout(_))` — the important
        // thing is that parser CONSTRUCTION (which is where the panic used
        // to fire) succeeded and we get a normal parse result, not a panic.
        let _ = parser.run_inner(Args::from(&["raw", "--help"]));
    }

    #[test]
    fn test_raw_hex_data() {
        let cli = cli_parser()
            .run_inner(Args::from(&["raw", "0x01", "0x02", "0x03"]))
            .unwrap();
        if let Commands::Raw { data, decimal } = cli.command {
            assert_eq!(data, vec!["0x01", "0x02", "0x03"]);
            assert!(!decimal);
        } else {
            panic!("Expected Raw command");
        }
    }

    #[test]
    fn test_parse_category() {
        assert_eq!(parse_category("amp").unwrap(), EffectCategory::Amp);
        assert!(matches!(
            parse_category("kazoo"),
            Err(CliError::UnknownCategory(name)) if name == "kazoo"
        ));
    }

    #[test]
    fn test_raw_decimal_flag_before_data() {
        let cli = cli_parser()
            .run_inner(Args::from(&["raw", "--decimal", "1", "2", "3"]))
            .unwrap();
        if let Commands::Raw { data, decimal } = cli.command {
            assert!(decimal);
            assert_eq!(data, vec!["1", "2", "3"]);
        } else {
            panic!("Expected Raw command");
        }
    }
}

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

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
    pub midi_port: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
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
}

// Example usage in main.rs
#[cfg(feature = "example")]
fn example_main() {
    use clap::Parser;

    let cli = Cli::parse();

    println!("Device: {:?}", cli.device_name);
    println!("MIDI Port: {:?}", cli.midi_port);

    match cli.command {
        Commands::Devices => {
            println!("Listing available devices...");
        }
        Commands::Info => {
            println!("Showing device info...");
        }
        Commands::Preset { action } => match action {
            PresetAction::Show => println!("Showing current preset..."),
            PresetAction::Get(args) => {
                println!(
                    "Getting preset from slot: {:?}, saving to: {:?}",
                    args.slot, args.filename
                );
            }
            PresetAction::Send(args) => {
                println!(
                    "Sending preset from: {:?} to slot: {:?}",
                    args.filename, args.slot
                );
            }
        },
        Commands::Effect { effect, parameters } => {
            println!("Controlling effect: {}", effect);
            for param in parameters {
                println!("  Setting parameter: {}", param);
            }
        }
    }
}

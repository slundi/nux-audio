// Preset management: fetch from device and send preset files.

use crate::export::OutputFormat;
use clap::{Args, Subcommand};
use std::path::PathBuf;

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

pub async fn run(action: PresetAction, format: OutputFormat) {
    match action {
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
    }
}

use clap::Parser;
use std::fs::OpenOptions;

pub mod cli;
pub mod device;
pub mod effects;
pub mod error;
pub mod midi;
pub mod preset;
pub mod registry;
pub mod utils;

// type SysExResponseData = DeviceVersion | Preset | Partial<Preset>;
// type KnobEntry = [string, string, [number, number]?];

/// Get an async file writer for the `rustmission.log` file located in XDG state path.
pub fn configure_logger(xdg: &xdg::BaseDirectories) {
    match xdg.place_state_file("nux-audio.log") {
        Ok(path) => match OpenOptions::new().create(true).append(true).open(path) {
            Ok(file) => {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::INFO)
                    .with_level(true)
                    .with_writer(file)
                    .with_ansi(false)
                    .init();
            }
            Err(e) => {
                eprintln!("Cannot open log file: {e}");
            }
        },
        Err(e) => {
            eprintln!("Unable to create log file: {e}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().len() > 1 {
        let cli = cli::Cli::parse();
        cli.process().await;
        return Ok(());
    }
    let xdg_dirs = xdg::BaseDirectories::new();
    configure_logger(&xdg_dirs);

    // NUX NMP-03:NUX NMP-03 MIDI 1 28:0

    // let devices = crate::device::Device::detect();
    // if devices.is_empty() {
    //     log::warn!("No MIDI device found, exiting");
    // }

    Ok(())
}

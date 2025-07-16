use log::{self, error};

pub mod device;
pub mod effects;
pub mod midi;
pub mod nux_sys_ex_commands;
pub mod preset;
pub mod utils;

// type SysExResponseData = DeviceVersion | Preset | Partial<Preset>;
// type KnobEntry = [string, string, [number, number]?];

fn main() {
    let mut log_file = None;
    let xdg_dirs = xdg::BaseDirectories::with_prefix("nux-audio");
    match xdg_dirs.place_state_file("app.log") {
        Ok(path) => {
            if path.is_file() {
                let _ = std::fs::remove_file(path.clone());
            }
            log_file = Some(path);
        }
        Err(e) => {
            eprintln!("Unable to create log file: {e}");
            return;
        }
    }

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(fern::log_file(log_file.unwrap()).expect("Cannot initialize log file"))
        .apply()
        .expect("Cannot initialize logger");

    // crate::midi::print_inputs();
    // crate::midi::print_outputs();

    // NUX NMP-03:NUX NMP-03 MIDI 1 28:0

    let devices = crate::device::Device::detect();
    if devices.is_empty() {
        log::warn!("No MIDI device found, exiting");
        return;
    }
}

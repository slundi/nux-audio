// Device info query: send VERSION_REQUEST SysEx and display firmware + registry data.

use crate::device::extract_version;
use crate::midi::MidiManager;
use crate::registry::REGISTRY;
use nux_audio::nux_sys_ex_commands::DEVICE_VERSION_REQUEST;
use std::time::Duration;

const VERSION_HEADER: [u8; 4] = [0xF0, 0x43, 0x58, 0x10];
const TIMEOUT: Duration = Duration::from_secs(3);

pub async fn run(device_name: &Option<String>) {
    let (device, port_in, port_out) = match super::resolve_ports(device_name) {
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

    println!("Device:       {}", device.name);
    println!("Model:        {}", device.model);
    println!("Device ID:    0x{:02X}", device.device_id);
    println!("Firmware:     {firmware}");
    if let Some(spec) = REGISTRY.by_device_id(device.device_id) {
        println!("Presets:      {}", spec.preset_count);
        println!("Signal chain: {}", spec.signal_chain.join(" → "));
    }
}

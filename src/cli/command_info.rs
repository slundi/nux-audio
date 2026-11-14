// Device info query: handshake via the session and display firmware + registry data.

use super::error::CliError;
use crate::registry::REGISTRY;
use crate::session::NuxSession;

pub async fn run(device_name: Option<&str>) -> Result<(), CliError> {
    let session = NuxSession::connect(device_name).await?;
    let device = session.device();

    println!("Device:       {}", device.name);
    println!("Model:        {}", device.model);
    println!("Device ID:    0x{:02X}", device.device_id);
    println!("Firmware:     {}", session.firmware());
    println!("Hardware gen: {:?}", session.fw_gen());
    if let Some(spec) = REGISTRY.by_device_id(device.device_id) {
        println!("Presets:      {}", spec.preset_count);
        println!("Signal chain: {}", spec.signal_chain.join(" → "));
    }
    Ok(())
}

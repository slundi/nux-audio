// Device info query: send VERSION_REQUEST SysEx and display firmware + registry data.

use crate::registry::REGISTRY;

pub async fn run(device_name: &Option<String>) {
    let session = match super::connect_and_detect(device_name).await {
        Some(s) => s,
        None => return,
    };

    println!("Device:       {}", session.device.name);
    println!("Model:        {}", session.device.model);
    println!("Device ID:    0x{:02X}", session.device.device_id);
    println!("Firmware:     {}", session.version_str);
    println!("Hardware gen: {:?}", session.fw_gen);
    if let Some(spec) = REGISTRY.by_device_id(session.device.device_id) {
        println!("Presets:      {}", spec.preset_count);
        println!("Signal chain: {}", spec.signal_chain.join(" → "));
    }
}

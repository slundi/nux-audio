use nux_audio::MidiManager;

#[cfg(feature = "bt")]
pub async fn list_bluetooth_devices() {
    println!("\nBluetooth devices:");
    match crate::bluetooth::BluetoothScanner::new().await {
        Ok(mut scanner) => match scanner.initialize().await {
            Ok(_) => match scanner.scan_devices(std::time::Duration::from_secs(10), None).await {
                Ok(devices) => {
                    for device in devices {
                        println!(
                            "* {:?} (id={}, address={})",
                            device.name, device.id, device.address
                        );
                    }
                }
                Err(err) => println!("{err}"),
            },
            Err(err) => println!("{err}"),
        },
        Err(err) => println!("{err}"),
    }
}

pub async fn list() {
    println!("Listing available devices...");
    match MidiManager::new() {
        Ok(manager) => {
            let ports = manager.list_ports();
            println!("{} MIDI ports found", ports.len());
            // TODO: filter NUX devices from name
            for (port, name) in ports {
                println!("* {port}\t{name}");
            }
            #[cfg(feature = "bt")]
            list_bluetooth_devices().await;
        }
        Err(_) => eprintln!("Cannot list devices"),
    }
}

use std::time::Duration;

use nux_audio::MidiManager;

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
            println!("\nBluetooth devices:");
            match crate::bluetooth::BluetoothScanner::new().await {
                Ok(mut scanner) => match scanner.initialize().await {
                    Ok(_) => {
                        match scanner.scan_devices(Duration::from_secs(10), None).await {
                            Ok(devices) => {
                                for device in devices {
                                    println!(
                                        "* {:?} (id={}, address={})",
                                        device.name, device.id, device.address
                                    );
                                }
                            }
                            Err(err) => println!("{err}"),
                        }
                    }
                    Err(err) => println!("{err}"),
                },
                Err(err) => println!("{err}"),
            }
        }
        Err(_) => eprintln!("Cannot list devices"),
    }
}

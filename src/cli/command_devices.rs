use nux_audio::MidiManager;

pub fn list() {
    println!("Listing available devices...");
    match MidiManager::new() {
        Ok(manager) => {
            let ports = manager.list_ports();
            println!("{} MIDI ports found", ports.len());
            // TODO: filter NUX devices from name
            for (port, name) in ports {
                println!("* {port}\t{name}");
            }
        }
        Err(_) => eprintln!("Cannot list devices"),
    }
}

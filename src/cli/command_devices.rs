use nux_audio::MidiManager;

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub port: usize,
    pub name: String,
    pub is_output: bool,
}

pub fn list() -> Vec<Port> {
    let mut ports: Vec<Port> = Vec::new();
    match MidiManager::new() {
        Ok(manager) => {
            // TODO: filter NUX devices from name
            for (port, name) in manager.list_output_ports() {
                ports.push(Port {port, name, is_output: true});
            }
            for (port, name) in manager.list_input_ports() {
                ports.push(Port {port, name, is_output: false});
            }
        }
        Err(e) => {
            eprintln!("Cannot list MIDI output ports: {e}");
        }
    }
    ports
}

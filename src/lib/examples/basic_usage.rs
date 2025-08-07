// Basic usage example
use nux_audio::effects::*;
use nux_audio::midi::MidiManager;
use nux_audio::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                humantime::format_rfc3339(std::time::SystemTime::now()),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;

    log::info!("Starting NUX Audio Effects Demo");

    // Create effects
    let mut overdrive = Overdrive::new(70, 60, 55);
    let mut chorus = Chorus::new(40, 50, 30, 40);
    let mut delay = Delay::new(35, 25, 20, 50);
    let mut reverb = Reverb::new(ReverbType::Hall, 60, 40, 25, 15);
    let mut amp = Amp::new(AmpModel::Crunch, 65, 55, 60, 65, 50, 75);

    log::info!(
        "Created effects: {}, {}, {}, {}, {}",
        overdrive.name(),
        chorus.name(),
        delay.name(),
        reverb.name(),
        amp.name()
    );

    // Create a preset
    let mut preset = Preset::new("My Rock Preset".to_string(), DeviceType::MightyPlugPro);
    preset.add_effect(amp);
    preset.add_effect(overdrive);
    preset.add_effect(chorus);
    preset.add_effect(delay);
    preset.add_effect(reverb);

    // Generate SysEx data
    let sysex_data = preset.to_sysex()?;
    log::info!("Generated SysEx data: {} bytes", sysex_data.len());

    // Print SysEx as hex for debugging
    let hex_string: String = sysex_data
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(" ");
    log::info!("SysEx data: {}", hex_string);

    // Create and configure MIDI manager
    let mut midi_manager = MidiManager::new()?;

    // List available MIDI ports
    let ports = midi_manager.list_ports();
    log::info!("Available MIDI ports:");
    for (index, name) in &ports {
        log::info!("  {}: {}", index, name);
    }

    // If there are MIDI ports available, try to connect and send data
    if !ports.is_empty() {
        // Try to find a NUX device or use the first available port
        let port_index = ports
            .iter()
            .find(|(_, name)| {
                name.to_lowercase().contains("nux") || name.to_lowercase().contains("mighty")
            })
            .map(|(index, _)| *index)
            .unwrap_or(0);

        log::info!("Connecting to MIDI port: {}", ports[port_index].1);

        match midi_manager.connect(port_index) {
            Ok(_) => {
                log::info!("Connected successfully!");

                // Send the preset data
                midi_manager.send_sysex(&sysex_data)?;
                log::info!("Sent preset data to device");

                // Send individual parameter changes
                midi_manager.send_effect_parameter(
                    DeviceType::MightyPlugPro,
                    0x10, // Overdrive effect ID
                    0x02, // Drive parameter
                    80,   // New value
                )?;
                log::info!("Sent parameter change: Overdrive drive = 80");

                // Send preset change
                midi_manager.send_preset_change(DeviceType::MightyPlugPro, 1)?;
                log::info!("Changed to preset 1");
            }
            Err(e) => {
                log::warn!("Failed to connect to MIDI port: {}", e);
            }
        }
    } else {
        log::info!("No MIDI ports available - running in demo mode only");
    }

    // Demonstrate preset management
    let mut preset_bank = PresetBank::new("My Presets".to_string(), DeviceType::MightyPlugPro);

    // Add factory presets
    preset_bank.add_preset(
        0,
        PresetFactory::create_clean_preset(DeviceType::MightyPlugPro),
    );
    preset_bank.add_preset(
        1,
        PresetFactory::create_rock_preset(DeviceType::MightyPlugPro),
    );
    preset_bank.add_preset(
        2,
        PresetFactory::create_lead_preset(DeviceType::MightyPlugPro),
    );

    log::info!(
        "Created preset bank with {} presets",
        preset_bank.presets.len()
    );

    // List presets
    for (num, name) in preset_bank.list_presets() {
        log::info!("  Preset {}: {}", num, name);
    }

    // Save preset bank to file
    let config_dir = xdg::BaseDirectories::with_prefix("nux-audio")?;
    let preset_file = config_dir.place_config_file("presets.json")?;
    preset_bank.save_to_file(&preset_file)?;
    log::info!("Saved preset bank to: {}", preset_file.display());

    // Load preset bank from file
    let loaded_bank = PresetBank::load_from_file(&preset_file)?;
    log::info!(
        "Loaded preset bank: {} with {} presets",
        loaded_bank.name,
        loaded_bank.presets.len()
    );

    log::info!("Demo completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overdrive_parameters() {
        let mut overdrive = Overdrive::default();

        // Test parameter setting
        assert!(overdrive.set_parameter("drive", 75).is_ok());
        assert_eq!(overdrive.get_parameter("drive"), Some(75));

        // Test invalid parameter
        assert!(overdrive.set_parameter("invalid", 50).is_err());

        // Test out of range value
        assert!(overdrive.set_parameter("drive", 150).is_err());
    }

    #[test]
    fn test_amp_models() {
        let amp = Amp::new(AmpModel::Clean, 30, 50, 50, 50, 50, 70);
        assert_eq!(amp.effect_type(), EffectType::AmpClean);
        assert_eq!(amp.name(), "Clean Amp");
    }

    #[test]
    fn test_sysex_generation() {
        let overdrive = Overdrive::new(60, 50, 55);
        let sysex = overdrive.to_sysex(DeviceType::MightyPlug);

        assert!(sysex.is_ok());
        let data = sysex.unwrap();

        // Should start with NUX SysEx header
        assert_eq!(&data[0..5], &[0xF0, 0x00, 0x20, 0x7E, 0x01]);
        // Should end with 0xF7
        assert_eq!(data[data.len() - 1], 0xF7);
    }

    #[test]
    fn test_device_effect_support() {
        let mighty_plug = DeviceFactory::create_device(DeviceType::MightyPlug);
        let mighty_plug_pro = DeviceFactory::create_device(DeviceType::MightyPlugPro);

        // Basic device should support basic effects
        assert!(mighty_plug.effect_id(EffectType::Overdrive).is_some());
        assert!(mighty_plug.effect_id(EffectType::Chorus).is_some());

        // Pro device should support more effects
        assert!(mighty_plug_pro
            .effect_id(EffectType::TubeScreamer)
            .is_some());
        assert!(mighty_plug_pro.effect_id(EffectType::Rotary).is_some());
    }

    #[test]
    fn test_preset_serialization() {
        let preset_data = PresetFactory::create_rock_preset(DeviceType::MightyPlugPro);

        // Test JSON serialization
        let json = serde_json::to_string(&preset_data).unwrap();
        let deserialized: PresetData = serde_json::from_str(&json).unwrap();

        assert_eq!(preset_data.name, deserialized.name);
        assert_eq!(preset_data.device_type, deserialized.device_type);
        assert_eq!(preset_data.effects.len(), deserialized.effects.len());
    }

    #[test]
    fn test_reverb_types() {
        let mut reverb = Reverb::default();

        reverb.set_type(ReverbType::Cathedral);
        assert_eq!(reverb.get_type(), &ReverbType::Cathedral);

        // Test parameter conversion
        assert!(reverb.set_parameter("type", 4).is_ok());
        assert_eq!(reverb.get_parameter("type"), Some(4));
    }
}

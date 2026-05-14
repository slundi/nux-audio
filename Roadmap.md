# Roadmap

## Phase 1: Foundation & Discovery

* [ ] **Core Data Structures**: Implement the domain model defined in `Design.md` (Manufacturer, MidiDevice, Patch, Pedal).
* [ ] **Device Registry**: Port the YAML device definitions (`devices.yam`l, `effects.yaml`) into a built-in registry.
* [ ] **Hardware abstraction**: Setup `midir` for cross-platform MIDI port enumeration (USB/Virtual MIDI).
* [ ] **Handshake Protocol**: Implement the version query and device identification logic using the `58 00` SysEx command.

## Phase 2: Communication Engine

* [ ] **SysEx Templating**: Build a message generator using the templates provided in `devices.yaml` and `Misc.md`.
* [ ] **Real-time Control (CC)**: Implement parameter updates via MIDI CC using the mapping table for knobs in `MIDI_CC_assignements.md`
* [ ] **Asynchronous handling**: Use `tokio` to manage non-blocking MIDI input/output and handle device responses (Pattern matching for responses).

## Phase 3: Preset Management

* [ ] **Preset Fetcher**: Logic to query and parse the current active preset (Basic and Detail modes).
* [ ] **Bulk Operations**: Implement listing and downloading all presets based on the `preset_count` per device.
* [ ] **Serialization**: Create an export/import format (likely YAML) based on the `Patch` structure.

## Phase 5: CLI (stateless)

Commands like `nux set amp-gain 50` or `nux dump-preset > my_tone.yaml`

[ ] **CLI Mode**: Basic clap commands for quick actions: `nux get-preset`, `nux list`, `nux set-param <id> <val>`.

## Phase 6: TUI (stateful)

[ ] **Persistence**: Local configuration to remember last used device and default folders.
[ ] **TUI Dashboard**: Interactive terminal interface for visual pedalboard management, effect toggling, and real-time knob adjustment.

## Phase 6: Community & Extensibility

* [ ] **Error Handling**: Replace basic panic! with a proper Result flow across the library.
* [ ] **Testing**: Mock MIDI backend for CI/CD testing without physical hardware.
* [ ] **External Mapping Support**: Allow users to load custom YAML files for unsupported pedals.
* [ ] **Documentation**: Document the SysEx/CC mapping process for community contributors.

## Phase 7: GUI



## Old roadmap

* [x] Connect to a MIDI device
* [ ] Detect the device type
* [ ] Get the device version
* [ ] Get the current preset
* [ ] List device presets when applicable
* [ ] Get/export device preset when applicable
* [ ] Send/export device preset when applicable
* [ ] Apply preset when applicable
* [ ] Create all effects
* [ ] Control effect
* [ ] async calls to MIDI stuffs?

There is reverse engineering to do so there are unknowns.

### Unknown

For now I don't know if I'll make a Rust library and/or a CLI and/or a TUI.

I also would like to convert preset between various devices.

Various unknow features:
* [ ] Use XDG for the configuration, presets, IRs, log file
* [ ] Log on the console and/or a file
* [ ] Convert presets for another device
* [ ] MIDI CC mapping
* [ ] drum & loop
* [ ] conversion between various patch format when possible or lossy

### TUI ideas

- [ ] keyboard mapping
- [ ] startup: device choice
- [ ] board **Amp Academy** / **MLD Bass Preamp + DI**:
  - [ ] current preset
  - [ ] IR choice
  - [ ] select amp
  - [ ] amp control (`g` gain, `l` low/bass, `m` middle, `h` high/treble, `v`: master volume, `b` bias)
  - [ ] display IR
  - [ ] select scene (keys: `shift + s`, `shift + 1`, `shift + 2`, `shift + 3`)
  - [ ] effect configuration (`n` noise gate, `r` reverb, `d` delay, `e` equalizer, `x` efx, `c` compression, `i` IR, `a` amp), `shift` to enable/disable

### Dropped

* Connect to a MIDI device through BLE (Bluetooth Low Energy): the program will not handle USB or bluetooth connection. MIDIR lib can communicate with any MIDI connected devices so this is useless.


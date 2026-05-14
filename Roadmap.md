# Roadmap

## Phase 1: Foundation & Discovery

* [x] **Core Data Structures**: `Manufacturer`, `Device` (model, device_id, MIDI port maps), `Preset` (number, name, scene, effect_order, effects map), `EffectOption` / `KnobDef` / `KnobState`, `ControllerState` — all implemented in `src/`.
* [x] **Device Registry**: `devices.toml` embedded at compile time and parsed at startup into a `static REGISTRY: LazyLock<DeviceRegistry>` (`src/registry.rs`). Each entry carries `device_id`, `model_code`, `preset_count`, `midi_port_prefix`, and `signal_chain`. `Device::extract_supported()` now delegates to `REGISTRY.match_port()` (longest-prefix match) instead of hardcoded branches. Effect model catalogs remain compiled-in Rust; `effects.yaml` is still reference-only.
* [x] **Hardware abstraction**: `MidiManager` with `midir` for port enumeration, connect, send, disconnect on Linux/ALSA. `Device::detect()` matches MIDI port names against 6 supported Nux device models.
* [~] **Handshake Protocol**: `DEVICE_VERSION_REQUEST` (`58 00`) sent by `nux info`. `extract_version()` parses the response bytes. Response **not yet wired** into device state — the MIDI input callback prints only.

## Phase 2: Communication Engine

* [~] **SysEx Templating**: `build_sysex_message()` and `send_sysex()` / `send_sysex_raw()` implemented. All known MG-30 request/response byte patterns defined as constants in `nux_sys_ex_commands`. A dynamic template system driven by `devices.yaml` is not yet built.
* [~] **Real-time Control (CC)**: All CC numbers mapped in `KnobDef.ctrl` for every effect slot and model. A CC send function is **not yet implemented** — the data model is ready, the wire is not.
* [~] **Asynchronous handling**: `tokio` in `Cargo.toml`, `Cli::process()` is async. MIDI callbacks are synchronous; response routing and async message dispatch are not yet wired.

## Phase 3: Preset Management

* [~] **Preset Fetcher**: `extract_current_preset_basic_data()` parses preset number and scene from the basic response. `extract_effects()` routes a SysEx response to the correct `EffectOption` from static catalogs (skeleton). `extract_effects_order()` and `extract_name()` implemented. Detail parsing (tempo, parallel, full effects) not yet complete.
* [ ] **Bulk Operations**: Listing and downloading all presets not implemented.
* [ ] **Serialization**: Export/import format (YAML/JSON) not implemented.

## Phase 4: Effect Control

* [~] **Effect model catalog**: Full MG-30 static catalog complete — wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir, vol (100+ models with correct CC assignments).
* [ ] **CC send**: Wire `KnobDef.ctrl` to actual MIDI CC messages for real-time knob control.
* [ ] **Bypass toggle**: Send bypass CC per slot (CC numbers documented in each effect file header).
* [ ] **Model switch**: Send SysEx to change active model within a slot.

## Phase 5: CLI (stateless)

Commands like `nux set amp-gain 50` or `nux dump-preset > my_tone.yaml`

* [~] **CLI structure**: `clap`-based commands exist — `devices`, `info`, `preset show/get/send`, `effect`, `raw`. Most are stubs printing TODO messages. `raw` can send arbitrary hex/decimal SysEx. `info` sends the version request.
* [ ] **Functional commands**: Wire commands to actual MIDI operations (get preset, set knob, toggle effect, export preset).

## Phase 6: TUI (stateful)

* [ ] **Persistence**: Local configuration to remember last used device and default folders.
* [ ] **TUI Dashboard**: Interactive terminal interface for visual pedalboard management, effect toggling, and real-time knob adjustment.

## Phase 7: Community & Extensibility

* [~] **Error Handling**: `anyhow`/`thiserror` used throughout. Some panics remain (e.g. `midir_test`).
* [~] **Logging**: `tracing` + `tracing-subscriber` configured; XDG log file placement done in `configure_logger`.
* [ ] **Testing**: Mock MIDI backend for CI/CD testing without physical hardware.
* [ ] **External Mapping Support**: Allow users to load custom YAML files for unsupported pedals.
* [ ] **Documentation**: Document the SysEx/CC mapping process for community contributors.

## Phase 8: GUI

## Old roadmap

* [x] Connect to a MIDI device
* [~] Detect the device type — `Device::detect()` works for 6 Nux models; not yet used for routing
* [~] Get the device version — request sent, response not yet parsed into device state
* [~] Get the current preset — basic data and name parsers done; detail and effects parsing skeletal
* [ ] List device presets when applicable
* [ ] Get/export device preset when applicable
* [ ] Send/import device preset when applicable
* [ ] Apply preset when applicable
* [x] Create all effects — MG-30 static catalog complete
* [ ] Control effect
* [ ] async calls to MIDI

There is reverse engineering to do so there are unknowns.

### Unknown

For now the focus is on MG-30. Other Nux devices (Mighty series) may share some SysEx patterns.

Various unknown features:
* [x] Use XDG for the log file
* [ ] Use XDG for configuration and preset folders
* [ ] Convert presets for another device
* [x] MIDI CC mapping — numbers documented; send not yet implemented
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

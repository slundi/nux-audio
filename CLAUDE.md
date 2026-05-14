# Agent Context: Nux-Rust-CLI

## Project Identity
**Nux-Rust-CLI** is a cross-platform (primary focus: Linux) CLI and TUI tool written in Rust. It aims to provide full control over Nux audio devices (MG-30, Amp Academy, Mighty Plug series) using MIDI SysEx and CC protocols.

## Core Architecture
- **Library (`nux_audio`)**: Pure logic, protocol parsing, and device definitions.
    - Uses `midir` for MIDI I/O.
    - Uses `thiserror` for domain-specific error handling.
    - Driven by YAML definitions for easy community updates.
- **Binary (`nux-cli`)**: The user interface.
    - Uses `clap` for command parsing.
    - Uses `tokio` for async MIDI message handling and timeouts.
    - Uses `tracing` for logs and debugging MIDI traffic.

## Technical Standards
- **Memory Safety**: No `unsafe` code unless strictly required by MIDI backends.
- **Protocol**: 
    - Always prefer SysEx for state synchronization (fetching current settings).
    - Use MIDI CC for real-time adjustments (knobs, toggles).
    - Handle Manufacturer ID `0x43` (Yamaha/Nux context) and specific Nux headers `58 xx`.
- **Data Integrity**: Presets must be exported in a versioned YAML/JSON format to ensure forward compatibility when device firmware updates.

## Development Constraints
1. **Async First**: All MIDI interactions must be non-blocking using `tokio`.
2. **Modular Devices**: Do not hardcode device logic. Use the trait system to allow different "Drivers" for MG-series vs Mighty-series.
3. **Linux Priority**: Ensure compatibility with ALSA/Jack backends via `midir`.
4. **Safety**: Implement "Sanity Checks" on CC values (0-127) before sending to prevent device crashes or audio spikes.

## Development Workflow
After coding always
1. **Write tests** or update tests for new or any modified code.
2. **Format and lint**: run `cargo fmt && cargo clippy` and fix lint errors.
3. Suggest **Conventionnal commit** message.
4. **Update Roadmap.md** with completed items.

## Knowledge Base Reference
- `devices.yaml`: Source of truth for Device IDs and capabilities.
- `MIDI_CC_assignments.md`: Mapping of control numbers to parameters.
- `Misc.md`: Collection of reverse-engineered SysEx opcodes.
# nux-audio

CLI + TUI tool to manage NUX audio devices over MIDI on Linux (and macOS/Windows).

Control presets, effects, bypass states, and more — entirely from the terminal, no
proprietary NUX app required.

> **Early-stage (0.1.0):** core CLI commands and the interactive TUI are functional.
> MIDI integration into the TUI is still in progress. Contributions welcome!

Based on reverse-engineering work from [NuxOnline](https://github.com/Srijan619/nuxOnline)
and [Mightier Amp](https://github.com/tuntorius/mightier_amp).

## Supported Devices

| Device | Model Code | Status |
|--------|-----------|--------|
| NUX MG-30 Versatile Modeler | MG-30 | Active development |
| NUX Mighty Plug Pro | MP-3 | Partial |
| NUX Mighty Space | MS-1 | Partial |
| NUX Mighty Plug MP-2 | MP-2 | Partial |
| NUX Mighty Air | MA-1 | Partial |
| NUX Mighty Lite BT MKII | ML-2 | Partial |

Only devices connected via USB-MIDI are supported. Bluetooth MIDI is not supported on Linux.

## Installation

### From source (recommended)

1. Install the [Rust toolchain](https://www.rust-lang.org/tools/install) (stable, 1.85+)
2. Install ALSA development headers (Linux): `sudo apt install libasound2-dev` (Debian/Ubuntu) or `sudo pacman -S alsa-lib` (Arch)
3. Clone and build:

```shell
git clone https://codeberg.org/slundi/nux-audio
cd nux-audio
cargo build --release
# Binary is at: ./target/release/nux-cli
```

### NixOS / nix develop

```shell
git clone https://codeberg.org/slundi/nux-audio
cd nux-audio
nix develop    # enter the dev shell with all tools available
cargo build --release
```

## Quickstart

```shell
# List connected NUX devices
nux-cli devices

# Show current preset info (name, tempo, scenes)
nux-cli preset show

# Set amp gain to 80, tone to 50
nux-cli effect amp gain=80 tone=50

# Bypass / enable the delay slot
nux-cli bypass delay off
nux-cli bypass delay on

# Export current preset to YAML
nux-cli preset export -o my-preset.yaml

# Launch the interactive TUI
nux-cli tui
```

## Usage

```shell
# Display CLI help
nux-cli --help
nux-cli -h

# Launch the interactive TUI
nux-cli tui
```

## TUI Key Bindings

Press `?` inside the TUI to open the in-app help overlay.

### Global

| Key | Action |
|-----|--------|
| `F1` / `F2` / `F3` | Switch tabs (Presets / Live Board / About) |
| `d` | Open device picker |
| `l` | Open looper overlay |
| `D` | Open drum machine overlay |
| `?` | Open this help overlay |
| `q` | Quit |

### Presets tab

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Move preset selection |
| `Enter` | Toggle chain panel focus |
| `Tab` | Cycle scene A → B → C |
| `g` | Pull current preset from device |
| `s` | Send selected preset to device |

### Live Board tab

| Key | Action |
|-----|--------|
| `h` / `l` or `←` / `→` | Move chain slot focus |
| `Tab` | Cycle scene A → B → C |

### Overlays

| Key | Action |
|-----|--------|
| `Esc` | Close any overlay |
| `o` / `n` / `r` | Conflict dialog: Override / No / Rename |

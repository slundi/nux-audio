# nux-audio

Manage NUX audio devices.

This is a tool to manage your Nux devices 🎸. I own a various Nux products:
* [MG30](https://nuxaudio.com/product/mg30/)
* [Amp Academy](https://nuxaudio.com/product/ampacademy/)
* [MLD Bass Preamp + DI](https://nuxaudio.com/product/mldbasspreampdi/)
* [Mighty Plug Pro (MP3)](https://nuxaudio.com/product/mightyplugpro/)

I'm a Linux user so I want to manage those devices in my current OS.

I have based my work on 2 projects ([NuxOnline](https://github.com/Srijan619/nuxOnline) and [Mightier Amp](https://github.com/tuntorius/mightier_amp)).

This project is in early stage, so don't expect to manage tour device yet. But, if you want to help me you are welcome 🙂

## Installation

1. Install the [🦀 Rust toolchain](https://www.rust-lang.org/tools/install) if you don't have it
2. clone this repository
3. in the project folder, run `cargo build --release`, the binary file will be in `./target/release/nux-audio`

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

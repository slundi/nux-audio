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
```

## Roadmap

* [x] Connect to a MIDI device
* [ ] Connect to a MIDI device through BLE (Bluetooth Low Energy)
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

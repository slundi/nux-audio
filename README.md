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

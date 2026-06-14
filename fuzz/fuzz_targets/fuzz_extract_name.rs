//! Fuzz target: `Preset::extract_name`.
//!
//! Run with: `cargo +nightly fuzz run fuzz_extract_name`
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = nux_audio::preset::Preset::extract_name(data);
});

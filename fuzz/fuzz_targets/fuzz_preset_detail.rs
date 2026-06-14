//! Fuzz target: `Preset::extract_current_preset_detail_data`.
//!
//! Run with: `cargo +nightly fuzz run fuzz_preset_detail`
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = nux_audio::preset::Preset::extract_current_preset_detail_data(data);
});

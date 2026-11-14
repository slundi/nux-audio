//! Fuzz target: MG-30 preset detail decoders (GEN1 and GEN2 layouts).
//!
//! Run with: `cargo +nightly fuzz run fuzz_preset_detail`
#![no_main]
use libfuzzer_sys::fuzz_target;
use nux_audio::driver::mg30::{decode_detail_gen1, decode_detail_gen2};

fuzz_target!(|data: &[u8]| {
    let _ = decode_detail_gen1(data);
    let _ = decode_detail_gen2(data);
});

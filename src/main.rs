pub mod device;
pub mod effects;
pub mod midi;
pub mod nux_sys_ex_commands;
pub mod preset;
pub mod utils;

// type SysExResponseData = DeviceVersion | Preset | Partial<Preset>;
// type KnobEntry = [string, string, [number, number]?];

fn main() {
    // crate::midi::print_inputs();
    // crate::midi::print_outputs();

    // NUX NMP-03:NUX NMP-03 MIDI 1 28:0

    crate::midi::midir_test();
}

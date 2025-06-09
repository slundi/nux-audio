use rusb::UsbContext;

pub mod device;
pub mod effects;
pub mod midi;
pub mod preset;
pub mod nux_sys_ex_commands;
pub mod usb;
pub mod utils;

// type SysExResponseData = DeviceVersion | Preset | Partial<Preset>;
// type KnobEntry = [string, string, [number, number]?];

fn main() {
    let usb = rusb::Context::new().unwrap();
    let devices = usb.devices().unwrap();

    if devices.is_empty() {
        println!("No USB devices found.");
    }
    crate::usb::print_devices(&devices);

}

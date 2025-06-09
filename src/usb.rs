use rusb::{Context, DeviceDescriptor, DeviceList};

pub fn print_devices(devices: &DeviceList<Context>) {
    println!("| Bus | Device | ID        |");
    println!("|-----|--------|-----------|");
    for device in devices.iter() {
        let device_descriptor: DeviceDescriptor = match device.device_descriptor() {
            Ok(descriptor) => descriptor,
            Err(e) => {
                eprintln!("Error getting device descriptor: {}", e);
                continue;
            }
        };

        println!(
            "| {:03} |    {:03} | {:04x}:{:04x} |",
            device.bus_number(),
            device.address(),
            device_descriptor.vendor_id(),
            device_descriptor.product_id()
        );
        // https://www.usb.org/defined-class-codes
        // 01h Interface Audio
        // 03h Interface HID (Human Interface Device)


        // You can get more information if you open the device.
        // Opening the device often requires elevated permissions (e.g., sudo on Linux).
        // For simple listing, you usually don't need to open them.
        // match device.open() {
        //     Ok(handle) => {
        //         // Example: Get manufacturer string (requires opening the device)
        //         if let Ok(manufacturer) = handle.read_manufacturer_string_ascii(&device_descriptor) {
        //             println!("    Manufacturer: {}", manufacturer);
        //         }
        //         // Example: Get product string
        //         if let Ok(product) = handle.read_product_string_ascii(&device_descriptor) {
        //             println!("    Product: {}", product);
        //         }
        //         // You can also iterate through configurations, interfaces, and endpoints here.
        //     }
        //     Err(e) => {
        //         eprintln!("    Could not open device: {}", e);
        //     }
        // }
    }
}

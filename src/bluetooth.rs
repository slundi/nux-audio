use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use log::{error, info, warn};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tokio::time;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum BluetoothError {
    #[error("Bluetooth adapter not found")]
    AdapterNotFound,
    #[error("Bluetooth not available: {0}")]
    NotAvailable(String),
    #[error("Scan failed: {0}")]
    ScanFailed(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub id: String,
    pub name: Option<String>,
    pub address: String,
    pub rssi: Option<i16>,
    pub services: Vec<Uuid>,
    pub manufacturer_data: HashMap<u16, Vec<u8>>,
}

pub struct BluetoothScanner {
    manager: Manager,
    adapter: Option<Adapter>,
}

impl BluetoothScanner {
    /// Create a new Bluetooth scanner
    pub async fn new() -> Result<Self, BluetoothError> {
        let manager = Manager::new()
            .await
            .map_err(|e| BluetoothError::NotAvailable(e.to_string()))?;

        Ok(Self {
            manager,
            adapter: None,
        })
    }

    /// Initialize the Bluetooth adapter
    pub async fn initialize(&mut self) -> Result<(), BluetoothError> {
        let adapters = self
            .manager
            .adapters()
            .await
            .map_err(|e| BluetoothError::NotAvailable(e.to_string()))?;

        if adapters.is_empty() {
            return Err(BluetoothError::AdapterNotFound);
        }

        // Use the first available adapter
        self.adapter = Some(adapters.into_iter().next().unwrap());
        info!("Bluetooth adapter initialized");
        Ok(())
    }

    /// Check if Bluetooth is available and enabled
    pub async fn is_available(&self) -> bool {
        if let Some(adapter) = &self.adapter {
            // Try to get adapter info to check if it's working
            match adapter.adapter_info().await {
                Ok(_) => true,
                Err(e) => {
                    warn!("Bluetooth adapter not available: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Scan for nearby Bluetooth devices
    pub async fn scan_devices(
        &self,
        duration: Duration,
        filter: Option<ScanFilter>,
    ) -> Result<Vec<BluetoothDevice>, BluetoothError> {
        let adapter = self
            .adapter
            .as_ref()
            .ok_or(BluetoothError::AdapterNotFound)?;

        info!("Starting Bluetooth scan for {:?}", duration);

        // Start scanning
        adapter
            .start_scan(filter.unwrap_or_default())
            .await
            .map_err(|e| BluetoothError::ScanFailed(e.to_string()))?;

        // Wait for the specified duration
        time::sleep(duration).await;

        // Stop scanning
        adapter
            .stop_scan()
            .await
            .map_err(|e| BluetoothError::ScanFailed(e.to_string()))?;

        // Get discovered peripherals
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| BluetoothError::ScanFailed(e.to_string()))?;

        let mut devices = Vec::new();

        for peripheral in peripherals {
            if let Ok(Some(props)) = peripheral.properties().await {
                let device = BluetoothDevice {
                    id: peripheral.id().to_string(),
                    name: props.local_name,
                    address: props.address.to_string(),
                    rssi: props.rssi,
                    services: props.services,
                    manufacturer_data: props.manufacturer_data,
                };
                devices.push(device);
            }
        }

        info!("Found {} Bluetooth devices", devices.len());
        Ok(devices)
    }

    /// Scan specifically for NUX devices
    pub async fn scan_nux_devices(
        &self,
        duration: Duration,
    ) -> Result<Vec<BluetoothDevice>, BluetoothError> {
        let devices = self.scan_devices(duration, None).await?;

        // Filter for NUX devices
        let nux_devices: Vec<BluetoothDevice> = devices
            .into_iter()
            .filter(|device| {
                if let Some(name) = &device.name {
                    let name_upper = name.to_uppercase();
                    name_upper.contains("MIGHTY")
                        || name_upper.contains("NUX")
                        || name_upper.contains("PLUG")
                        || name_upper.contains("AIR")
                        || name_upper.contains("SPACE")
                } else {
                    false
                }
            })
            .collect();

        info!("Found {} NUX devices", nux_devices.len());
        Ok(nux_devices)
    }

    /// Scan for devices with a specific service UUID (e.g., MIDI)
    pub async fn scan_for_service(
        &self,
        service_uuid: Uuid,
        duration: Duration,
    ) -> Result<Vec<BluetoothDevice>, BluetoothError> {
        let filter = ScanFilter {
            services: vec![service_uuid],
        };

        self.scan_devices(duration, Some(filter)).await
    }

    /// Get detailed information about a specific device
    pub async fn get_device_info(
        &self,
        device_id: &str,
    ) -> Result<Option<BluetoothDevice>, BluetoothError> {
        let adapter = self
            .adapter
            .as_ref()
            .ok_or(BluetoothError::AdapterNotFound)?;

        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| BluetoothError::ScanFailed(e.to_string()))?;

        for peripheral in peripherals {
            if peripheral.id().to_string() == device_id {
                if let Ok(Some(props)) = peripheral.properties().await {
                    return Ok(Some(BluetoothDevice {
                        id: peripheral.id().to_string(),
                        name: props.local_name,
                        address: props.address.to_string(),
                        rssi: props.rssi,
                        services: props.services,
                        manufacturer_data: props.manufacturer_data,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Connect to a Bluetooth device
    pub async fn connect_device(&self, device_id: &str) -> Result<(), BluetoothError> {
        let adapter = self
            .adapter
            .as_ref()
            .ok_or(BluetoothError::AdapterNotFound)?;

        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| BluetoothError::ScanFailed(e.to_string()))?;

        for peripheral in peripherals {
            if peripheral.id().to_string() == device_id {
                peripheral
                    .connect()
                    .await
                    .map_err(|e| BluetoothError::ConnectionFailed(e.to_string()))?;

                info!("Connected to device: {}", device_id);
                return Ok(());
            }
        }

        Err(BluetoothError::ConnectionFailed(
            "Device not found".to_string(),
        ))
    }

    /// Disconnect from a Bluetooth device
    pub async fn disconnect_device(&self, device_id: &str) -> Result<(), BluetoothError> {
        let adapter = self
            .adapter
            .as_ref()
            .ok_or(BluetoothError::AdapterNotFound)?;

        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| BluetoothError::ScanFailed(e.to_string()))?;

        for peripheral in peripherals {
            if peripheral.id().to_string() == device_id {
                peripheral
                    .disconnect()
                    .await
                    .map_err(|e| BluetoothError::ConnectionFailed(e.to_string()))?;

                info!("Disconnected from device: {}", device_id);
                return Ok(());
            }
        }

        Ok(()) // Not an error if device wasn't connected
    }
}

// Utility functions
impl BluetoothDevice {
    /// Check if this device is likely a NUX audio device
    pub fn is_nux_device(&self) -> bool {
        if let Some(name) = &self.name {
            let name_upper = name.to_uppercase();
            name_upper.contains("MIGHTY")
                || name_upper.contains("NUX")
                || name_upper.contains("PLUG")
                || name_upper.contains("AIR")
                || name_upper.contains("SPACE")
        } else {
            false
        }
    }

    /// Check if this device supports MIDI over Bluetooth LE
    pub fn supports_midi_ble(&self) -> bool {
        // Standard MIDI BLE service UUID
        let midi_service_uuid = Uuid::parse_str("03B80E5A-EDE8-4B33-A751-6CE34EC4C700").unwrap();
        self.services.contains(&midi_service_uuid)
    }

    /// Get a display name for the device
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("Unknown Device ({})", &self.address[..8]))
    }

    /// Get signal strength description
    pub fn signal_strength_description(&self) -> String {
        match self.rssi {
            Some(rssi) if rssi >= -30 => "Excellent".to_string(),
            Some(rssi) if rssi >= -50 => "Good".to_string(),
            Some(rssi) if rssi >= -70 => "Fair".to_string(),
            Some(rssi) if rssi >= -90 => "Weak".to_string(),
            Some(_) => "Very Weak".to_string(),
            None => "Unknown".to_string(),
        }
    }
}

// Example usage
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     env_logger::init();

//     let mut scanner = BluetoothScanner::new().await?;
//     scanner.initialize().await?;

//     if !scanner.is_available().await {
//         eprintln!("Bluetooth is not available on this system");
//         return Ok(());
//     }

//     println!("Scanning for Bluetooth devices...");

//     // Scan for all devices for 10 seconds
//     let devices = scanner.scan_devices(Duration::from_secs(10), None).await?;

//     println!("\nFound {} devices:", devices.len());
//     for device in &devices {
//         println!("  {} ({})", device.display_name(), device.address);
//         if let Some(rssi) = device.rssi {
//             println!(
//                 "    Signal: {} dBm ({})",
//                 rssi,
//                 device.signal_strength_description()
//             );
//         }
//         if device.is_nux_device() {
//             println!("    🎸 NUX Audio Device detected!");
//         }
//         if device.supports_midi_ble() {
//             println!("    🎵 MIDI BLE supported");
//         }
//         println!("    Connectable: {}", device.is_connectable);
//         if !device.services.is_empty() {
//             println!("    Services: {} available", device.services.len());
//         }
//         println!();
//     }

//     // Scan specifically for NUX devices
//     println!("Scanning specifically for NUX devices...");
//     let nux_devices = scanner.scan_nux_devices(Duration::from_secs(5)).await?;

//     if nux_devices.is_empty() {
//         println!("No NUX devices found.");
//     } else {
//         println!("NUX devices found:");
//         for device in nux_devices {
//             println!("  🎸 {} ({})", device.display_name(), device.address);
//             println!("     Signal: {}", device.signal_strength_description());
//             println!("     Connectable: {}", device.is_connectable);

//             // Try to connect to the first NUX device found
//             if device.is_connectable {
//                 println!("     Attempting to connect...");
//                 match scanner.connect_device(&device.id).await {
//                     Ok(_) => {
//                         println!("     ✅ Connected successfully!");

//                         // Wait a moment, then disconnect
//                         tokio::time::sleep(Duration::from_secs(2)).await;

//                         if let Err(e) = scanner.disconnect_device(&device.id).await {
//                             println!("     ⚠️  Error disconnecting: {}", e);
//                         } else {
//                             println!("     ✅ Disconnected successfully!");
//                         }
//                     }
//                     Err(e) => println!("     ❌ Connection failed: {}", e),
//                 }
//             }
//             println!();
//         }
//     }

//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scanner_creation() {
        let scanner = BluetoothScanner::new().await;
        assert!(scanner.is_ok());
    }

    #[test]
    fn test_nux_device_detection() {
        let device = BluetoothDevice {
            id: "test".to_string(),
            name: Some("MIGHTY PLUG".to_string()),
            address: "00:11:22:33:44:55".to_string(),
            rssi: Some(-50),
            services: Vec::new(),
            manufacturer_data: HashMap::new(),
        };

        assert!(device.is_nux_device());
        assert_eq!(device.display_name(), "MIGHTY PLUG");
        assert_eq!(device.signal_strength_description(), "Good");
    }

    #[test]
    fn test_midi_service_detection() {
        let midi_service_uuid = Uuid::parse_str("03B80E5A-EDE8-4B33-A751-6CE34EC4C700").unwrap();

        let device = BluetoothDevice {
            id: "test".to_string(),
            name: Some("MIDI Device".to_string()),
            address: "00:11:22:33:44:55".to_string(),
            rssi: None,
            services: vec![midi_service_uuid],
            manufacturer_data: HashMap::new(),
        };

        assert!(device.supports_midi_ble());
    }
}

use serialport::{available_ports, SerialPortType};
use crate::PortInfo;

/// List all available serial ports on the system
pub fn list_ports() -> Result<Vec<PortInfo>, String> {
    let ports = available_ports().map_err(|e| e.to_string())?;

    let port_infos: Vec<PortInfo> = ports
        .into_iter()
        .map(|p| {
            let (description, port_type) = match p.port_type {
                SerialPortType::UsbPort(info) => {
                    let desc = info.product.unwrap_or_else(|| "USB Serial".to_string());
                    (Some(desc), "usb".to_string())
                }
                SerialPortType::PciPort => (Some("PCI Serial".to_string()), "pci".to_string()),
                SerialPortType::BluetoothPort => {
                    (Some("Bluetooth Serial".to_string()), "bluetooth".to_string())
                }
                SerialPortType::Unknown => (None, "unknown".to_string()),
            };

            PortInfo {
                name: p.port_name,
                description,
                port_type,
            }
        })
        .collect();

    Ok(port_infos)
}

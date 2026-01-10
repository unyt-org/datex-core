use crate::std_sync::Mutex;
use crate::stdlib::{future::Future, pin::Pin, sync::Arc, time::Duration};
use core::prelude::rust_2024::*;
use core::result::Result;
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use super::serial_common::{SerialError, SerialInterfaceSetupData};

use crate::{
    task::spawn,
    task::spawn_blocking,
};
use log::{debug, error, warn};
use serialport::SerialPort;
use tokio::sync::Notify;
use crate::network::com_interfaces::com_interface::implementation::ComInterfaceImplementation;
use datex_macros::{com_interface, create_opener};
use crate::network::com_interfaces::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::implementation::ComInterfaceFactory;
use crate::network::com_interfaces::com_interface::properties::{InterfaceDirection, InterfaceProperties};
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::com_interface::socket_manager::ComInterfaceSocketManager;
use crate::network::com_interfaces::com_interface::state::ComInterfaceState;

pub struct SerialNativeInterface {
    com_interface: Rc<ComInterface>,
    shutdown_signal: Arc<Notify>,
    port: Arc<Mutex<Box<dyn SerialPort + Send>>>,
}

impl SerialNativeInterface {
    const TIMEOUT: Duration = Duration::from_millis(1000);
    const BUFFER_SIZE: usize = 1024;
    const DEFAULT_BAUD_RATE: u32 = 115200;

    pub fn get_available_ports() -> Vec<String> {
        serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|port| port.port_name.into())
            .collect()
    }

    pub fn new(
        port_name: &str,
        com_interface: Rc<ComInterface>,
    ) -> Result<SerialNativeInterface, SerialError> {
        Self::new_with_baud_rate(port_name, Self::DEFAULT_BAUD_RATE, com_interface)
    }
    // Allow to open interface with a configured port
    pub fn new_with_port(
        port: Box<dyn SerialPort + Send>,
        com_interface: Rc<ComInterface>,
    ) -> Result<SerialNativeInterface, SerialError> {
        let interface = SerialNativeInterface {
            shutdown_signal: Arc::new(Notify::new()),
            port: Arc::new(Mutex::new(port)),
            com_interface,
        };
        Ok(interface)
    }
    pub fn new_with_baud_rate(
        port_name: &str,
        baud_rate: u32,
        com_interface: Rc<ComInterface>,
    ) -> Result<SerialNativeInterface, SerialError> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Self::TIMEOUT)
            .open()
            .map_err(|_| SerialError::PortNotFound)?;
        Self::new_with_port(port, com_interface)
    }

    fn open(&self) -> Result<(), SerialError> {
        let state = self.com_interface.state();
        let port = self.port.clone();

        let (socket_uuid, mut sender) = self
            .com_interface
            .borrow()
            .socket_manager().lock().unwrap()
            .create_and_init_socket(InterfaceDirection::InOut, 1);


        let shutdown_signal = self.shutdown_signal.clone();
        spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_signal.notified() => {
                        warn!("Shutting down serial task...");
                        break;
                    },
                    result = spawn_blocking({
                        let port = port.clone();
                        move || {
                            let mut buffer = [0u8; Self::BUFFER_SIZE];
                            match port.try_lock().unwrap().read(&mut buffer) {
                                Ok(n) if n > 0 => Some(buffer[..n].to_vec()),
                                _ => None,
                            }
                        }
                    }) => {
                        match result {
                            Ok(Some(incoming)) => {
                                let size = incoming.len();
                                sender.start_send(incoming).unwrap();
                                debug!(
                                    "Received data from serial port: {size}"
                                );
                            }
                            _ => {
                                error!("Serial read error or shutdown");
                                break;
                            }
                        }
                    }
                }
            }
            // FIXME #212 add reconnect logic (close gracefully and reopen)
            state.try_lock().unwrap().set(ComInterfaceState::Destroyed);
            warn!("Serial socket closed");
        });
        Ok(())
    }
}

impl ComInterfaceFactory for SerialNativeInterface {
    type SetupData = SerialInterfaceSetupData;
    fn create(
        setup_data: Self::SetupData,
        com_interface: Rc<ComInterface>,
    ) -> Result<SerialNativeInterface, ComInterfaceError> {
        if let Some(port) = setup_data.port_name {
            if port.is_empty() {
                return Err(ComInterfaceError::InvalidSetupData);
            }
            SerialNativeInterface::new(&port, com_interface)
                .map_err(|_| ComInterfaceError::InvalidSetupData)
        } else {
            Err(ComInterfaceError::InvalidSetupData)
        }
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "serial".to_string(),
            channel: "serial".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 100,
            ..InterfaceProperties::default()
        }
    }
}

impl ComInterfaceImplementation for SerialNativeInterface {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let port = self.port.clone();
        Box::pin(async move {
            // FIXME #213 improve the lifetime issue here to avoid cloning the block twice
            let block = block.to_vec();
            let result = spawn_blocking(move || {
                let mut locked = port.try_lock().unwrap();
                locked.write_all(block.as_slice()).is_ok()
            })
            .await;
            result.unwrap_or(false)
        })
    }

    fn handle_close<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let shutdown_signal = self.shutdown_signal.clone();
        Box::pin(async move {
            shutdown_signal.notified().await;
            true
        })
    }

    fn get_properties(&self) -> InterfaceProperties {
        Self::get_default_properties()
    }

    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output=bool> + 'a>> {
        Box::pin(async move { self.open().is_ok() })
    }
}

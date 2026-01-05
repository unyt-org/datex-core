use crate::std_sync::Mutex;
use crate::stdlib::{future::Future, pin::Pin, sync::Arc, time::Duration};
use core::prelude::rust_2024::*;
use core::result::Result;

use super::serial_common::{SerialError, SerialInterfaceSetupData};
use crate::network::com_interfaces::com_interface::{
    ComInterfaceError, ComInterfaceFactory,
};
use crate::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{ComInterface, ComInterfaceInfo, ComInterfaceSockets},
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
    },
    set_sync_opener,
};
use crate::{
    network::com_interfaces::{
        com_interface::ComInterfaceState, socket_provider::SingleSocketProvider,
    },
    task::spawn,
    task::spawn_blocking,
};
use log::{debug, error, warn};
use serialport::SerialPort;
use tokio::sync::Notify;

pub struct SerialNativeInterface {
    info: ComInterfaceInfo,
    shutdown_signal: Arc<Notify>,
    port: Arc<Mutex<Box<dyn SerialPort + Send>>>,
}
impl SingleSocketProvider for SerialNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}

use datex_macros::{com_interface, create_opener};
#[com_interface]
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

    pub fn new(port_name: &str) -> Result<SerialNativeInterface, SerialError> {
        Self::new_with_baud_rate(port_name, Self::DEFAULT_BAUD_RATE)
    }
    // Allow to open interface with a configured port
    pub fn new_with_port(
        port: Box<dyn SerialPort + Send>,
    ) -> Result<SerialNativeInterface, SerialError> {
        let interface = SerialNativeInterface {
            shutdown_signal: Arc::new(Notify::new()),
            info: ComInterfaceInfo::new(),
            port: Arc::new(Mutex::new(port)),
        };
        Ok(interface)
    }
    pub fn new_with_baud_rate(
        port_name: &str,
        baud_rate: u32,
    ) -> Result<SerialNativeInterface, SerialError> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Self::TIMEOUT)
            .open()
            .map_err(|_| SerialError::PortNotFound)?;
        Self::new_with_port(port)
    }

    #[create_opener]
    fn open(&mut self) -> Result<(), SerialError> {
        let state = self.get_info().state.clone();
        let port = self.port.clone();
        let socket = ComInterfaceSocket::init(
            self.get_uuid().clone(),
            InterfaceDirection::InOut,
            1,
        );
        let receive_queue = socket.get_receive_queue().clone();
        self.add_socket(Arc::new(Mutex::new(socket)));
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
                                receive_queue.try_lock().unwrap().extend(incoming);
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

impl ComInterfaceFactory<SerialInterfaceSetupData> for SerialNativeInterface {
    fn create(
        setup_data: SerialInterfaceSetupData,
    ) -> Result<SerialNativeInterface, ComInterfaceError> {
        if let Some(port) = setup_data.port_name {
            if port.is_empty() {
                return Err(ComInterfaceError::InvalidSetupData);
            }
            SerialNativeInterface::new(&port)
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

impl ComInterface for SerialNativeInterface {
    fn send_block<'a>(
        &'a mut self,
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

    fn init_properties(&self) -> InterfaceProperties {
        Self::get_default_properties()
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let shutdown_signal = self.shutdown_signal.clone();
        Box::pin(async move {
            shutdown_signal.notified().await;
            true
        })
    }
    delegate_com_interface_info!();
    set_sync_opener!(open);
}

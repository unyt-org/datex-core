use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceInfo, ComInterfaceSockets,
            ComInterfaceUUID,
        },
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
    },
};
use crate::{
    network::com_interfaces::{
        com_interface::ComInterfaceState, socket_provider::SingleSocketProvider,
    },
    tasks::spawn,
    tasks::spawn_blocking,
};
use log::{debug, error, warn};
use serialport::SerialPort;
use tokio::sync::Notify;

use super::serial_common::SerialError;

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

    // Allow to open interface with a configured port
    pub fn open_with_port(
        port: Box<dyn SerialPort + Send>,
    ) -> Result<SerialNativeInterface, SerialError> {
        let mut interface = SerialNativeInterface {
            shutdown_signal: Arc::new(Notify::new()),
            info: ComInterfaceInfo::new(),
            port: Arc::new(Mutex::new(port)),
        };
        interface.start()?;
        Ok(interface)
    }
    pub fn open(port_name: &str) -> Result<SerialNativeInterface, SerialError> {
        Self::open_with_baud_rate(port_name, Self::DEFAULT_BAUD_RATE)
    }
    pub fn open_with_baud_rate(
        port_name: &str,
        baud_rate: u32,
    ) -> Result<SerialNativeInterface, SerialError> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Self::TIMEOUT)
            .open()
            .map_err(|_| SerialError::PortNotFound)?;
        Self::open_with_port(port)
    }

    fn start(&mut self) -> Result<(), SerialError> {
        let state = self.get_info().get_state();
        let port = self.port.clone();
        let socket = ComInterfaceSocket::new(
            self.get_uuid().clone(),
            InterfaceDirection::IN_OUT,
            1,
        );
        let receive_queue = socket.get_receive_queue().clone();
        self.add_socket(Arc::new(Mutex::new(socket)));
        let shutdown_signal = self.shutdown_signal.clone();
        spawn(async move {
            state
                .lock()
                .unwrap()
                .set_state(ComInterfaceState::Connected);
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
                            match port.lock().unwrap().read(&mut buffer) {
                                Ok(n) if n > 0 => Some(buffer[..n].to_vec()),
                                _ => None,
                            }
                        }
                    }) => {
                        match result {
                            Ok(Some(incoming)) => {
                                let size = incoming.len();
                                receive_queue.lock().unwrap().extend(incoming);
                                debug!(
                                    "Received data from serial port: {}",
                                    size
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
            state.lock().unwrap().set_state(ComInterfaceState::Closed);
            warn!("Serial socket closed");
        });
        Ok(())
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
            // FIXME improve the lifetime issue here to avoid cloning the block twice
            let block = block.to_vec();
            let result = spawn_blocking(move || {
                let mut locked = port.lock().unwrap();
                locked.write_all(block.as_slice()).is_ok()
            })
            .await;
            result.unwrap_or(false)
        })
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "serial".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 100,
            ..InterfaceProperties::default()
        }
    }
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let shutdown_signal = self.shutdown_signal.clone();
        Box::pin(async move {
            shutdown_signal.notified().await;
            self.set_state(ComInterfaceState::Closed);
            true
        })
    }
    delegate_com_interface_info!();
}

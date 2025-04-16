use std::{
    collections::HashMap,
    future::Future,
    io::ErrorKind,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::network::com_interfaces::{
    com_interface::ComInterfaceState, socket_provider::SingleSocketProvider,
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
        socket_provider::MultipleSocketProvider,
    },
};
use futures::{select, FutureExt};
use futures_timer::Delay;
use log::{debug, error, info, warn};
use matchbox_socket::{PeerId, PeerState, RtcIceServerConfig, WebRtcSocket};
use serialport::SerialPort;
use tokio::{spawn, sync::Notify};
use url::Url;

use super::serial_common::SerialError;

pub struct SerialNativeInterface {
    info: ComInterfaceInfo,
    shutdown_signal: Option<Arc<Notify>>,
    port: Arc<Mutex<Box<dyn SerialPort + Send>>>,
}
impl SingleSocketProvider for SerialNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}
impl SerialNativeInterface {
    async fn open(address: &str) -> Result<SerialNativeInterface, SerialError> {
        let port = serialport::new(address, 115200)
            .timeout(Duration::from_millis(1000))
            .open()
            .map_err(|_| SerialError::PortNotFound)?;

        let mut interface = SerialNativeInterface {
            shutdown_signal: None,
            info: ComInterfaceInfo::new(),
            port: Arc::new(Mutex::new(port)),
        };
        let _ = interface.start();
        Ok(interface)
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
        spawn(async move {
            state
                .lock()
                .unwrap()
                .set_state(ComInterfaceState::Connected);
            let mut buffer = [0u8; 1024];
            loop {
                match port.lock().unwrap().read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        let incoming = &buffer[..n];
                        receive_queue.lock().unwrap().extend(incoming);
                        debug!(
                            "Received data from serial port: {:?}",
                            incoming
                        );
                    }
                    Ok(_) => continue,
                    Err(ref e) if e.kind() == ErrorKind::TimedOut => continue,
                    Err(e) => {
                        error!("Serial read error: {}", e);
                        break;
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
            let block = block.to_vec();
            let result = tokio::task::spawn_blocking(move || {
                let mut locked = port.lock().unwrap();
                locked.write_all(&block.as_slice()).is_ok()
            })
            .await;
            match result {
                Ok(success) => success,
                Err(_) => false,
            }
        })
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "serial".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let shutdown_signal = self.shutdown_signal.clone();
        Box::pin(async move {
            if shutdown_signal.is_some() {
                shutdown_signal.unwrap().notified().await;
            }
            true
        })
    }
    delegate_com_interface_info!();
}

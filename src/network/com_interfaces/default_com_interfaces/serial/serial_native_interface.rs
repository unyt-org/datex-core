use std::{
    collections::HashMap,
    future::Future,
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
use tokio::{spawn, sync::Notify};
use url::Url;

use super::serial_common::SerialError;

pub struct SerialNativeInterface {
    info: ComInterfaceInfo,
    shutdown_signal: Option<Arc<Notify>>,
}
impl SingleSocketProvider for SerialNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}
impl SerialNativeInterface {
    async fn open(
        address: &str,
        ice_server_config: Option<RtcIceServerConfig>,
        use_reliable_connection: bool,
    ) -> Result<SerialNativeInterface, SerialError> {
        let mut interface = SerialNativeInterface {
            shutdown_signal: None,
            info: ComInterfaceInfo::new(),
        };
        interface.start(use_reliable_connection).await?;
        Ok(interface)
    }

    async fn start(
        &mut self,
        use_reliable_connection: bool,
    ) -> Result<(), SerialError> {
        let state = self.get_info().get_state();
        spawn(async move {
            state.lock().unwrap().set_state(ComInterfaceState::Closed);
            warn!("WebRTC socket closed");
        });
        Ok(())
    }
}

impl ComInterface for SerialNativeInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        return Box::pin(async { false });
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

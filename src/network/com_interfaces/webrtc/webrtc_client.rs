use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use datex_core::{
    network::com_interfaces::{
        com_interface::{ComInterface, ComInterfaceSockets, ComInterfaceUUID},
        com_interface_properties::InterfaceProperties,
        com_interface_socket::{ComInterfaceSocketUUID, SocketState},
        webrtc::webrtc_common::WebRTCError,
    },
    utils::uuid::UUID,
};
use log::{debug, error, info};
use url::Url;

pub struct WebRTCClientJSInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    pub com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,

    pub state: Rc<RefCell<SocketState>>,
}

impl WebRTCClientJSInterface {
    pub async fn open(
        address: &str,
    ) -> Result<WebRTCClientJSInterface, WebRTCError> {
        let uuid = ComInterfaceUUID(UUID::new());
        let com_interface_sockets =
            Arc::new(Mutex::new(ComInterfaceSockets::default()));
        let address =
            Url::parse(address).map_err(|_| WebRTCError::InvalidURL)?;

        let mut interface = WebRTCClientJSInterface {
            address,
            uuid,
            com_interface_sockets,
            state: Rc::new(RefCell::new(SocketState::Closed)),
        };
        interface.start().await?;
        Ok(interface)
    }
    async fn start(&mut self) -> Result<(), WebRTCError> {
        let address = self.address.clone();
        info!(
            "Connecting to WebSocket server at {}",
            address.host_str().unwrap()
        );
        let (mut socket, loop_fut) =
            WebRtcSocket::new_reliable("ws://localhost:3536/");

        Ok(())
    }
}
impl ComInterface for WebRTCClientJSInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move {
            debug!("Sending block: {:?}", block);

            // TODO
            true
        })
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "webrtc".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    fn get_uuid(&self) -> &ComInterfaceUUID {
        &self.uuid
    }

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
}

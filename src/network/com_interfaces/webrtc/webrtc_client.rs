use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    network::com_interfaces::{
        com_interface::{ComInterface, ComInterfaceSockets, ComInterfaceUUID},
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{
            ComInterfaceSocket, ComInterfaceSocketUUID, SocketState,
        },
        webrtc::webrtc_common::WebRTCError,
    },
    utils::uuid::UUID,
};
use futures_util::FutureExt;
use log::{debug, error, info};
use matchbox_socket::{PeerState, WebRtcChannel, WebRtcSocket};
use tokio::spawn;
use url::Url;

pub struct WebRTCClientInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    pub com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    socket: Option<Arc<Mutex<WebRtcSocket>>>,
    // pub state: Rc<RefCell<SocketState>>,
}

impl WebRTCClientInterface {
    const CHANNEL_ID: usize = 0;

    pub async fn open(
        address: &str,
    ) -> Result<WebRTCClientInterface, WebRTCError> {
        let uuid = ComInterfaceUUID(UUID::new());
        let com_interface_sockets =
            Arc::new(Mutex::new(ComInterfaceSockets::default()));
        let address =
            Url::parse(address).map_err(|_| WebRTCError::InvalidURL)?;

        let mut interface = WebRTCClientInterface {
            address,
            uuid,
            socket: None,
            com_interface_sockets,
            // state: Rc::new(RefCell::new(SocketState::Closed)),
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
        let (socket, loop_fut) = WebRtcSocket::new_reliable(address);
        let socket = Arc::new(Mutex::new(socket));
        self.socket = Some(socket.clone());
        let interface_uuid = self.uuid.clone();

        spawn(async move {
            let socket = socket.as_ref();
            let mut socket = socket.lock().unwrap();
            loop {
                for (peer, state) in socket.update_peers() {
                    match state {
                        PeerState::Connected => {
                            let socket = ComInterfaceSocket::new(
                                interface_uuid.clone(),
                                InterfaceDirection::IN_OUT,
                                1,
                            );

                            info!("Peer joined: {peer}");
                            let packet = "hello friend!"
                                .as_bytes()
                                .to_vec()
                                .into_boxed_slice();
                            // socket.channel_mut(CHANNEL_ID).send(packet, peer);
                        }
                        PeerState::Disconnected => {
                            info!("Peer left: {peer}");
                        }
                    }
                }

                for (peer, packet) in
                    socket.channel_mut(Self::CHANNEL_ID).receive()
                {
                    let message = String::from_utf8_lossy(&packet);
                    info!("Message from {peer}: {message:?}");
                }
            }
        });
        Ok(())
    }

    fn update(&self) {
        let interface_uuid = self.uuid.clone();
        let socket = self.socket.as_ref().unwrap();
        let mut socket = socket.lock().unwrap();
        loop {
            for (peer, state) in socket.update_peers() {
                match state {
                    PeerState::Connected => {
                        let socket = ComInterfaceSocket::new(
                            interface_uuid.clone(),
                            InterfaceDirection::IN_OUT,
                            1,
                        );

                        info!("Peer joined: {peer}");
                        let packet = "hello friend!"
                            .as_bytes()
                            .to_vec()
                            .into_boxed_slice();
                        // socket.channel_mut(CHANNEL_ID).send(packet, peer);
                    }
                    PeerState::Disconnected => {
                        info!("Peer left: {peer}");
                    }
                }
            }

            for (peer, packet) in socket.channel_mut(Self::CHANNEL_ID).receive()
            {
                let message = String::from_utf8_lossy(&packet);
                info!("Message from {peer}: {message:?}");
            }
        }
    }
}
impl ComInterface for WebRTCClientInterface {
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

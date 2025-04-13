use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    network::com_interfaces::{
        com_interface::{ComInterface, ComInterfaceSockets, ComInterfaceUUID},
        com_interface_properties::{InterfaceDirection, InterfaceProperties},
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
        socket_provider::MultipleSocketProvider,
        webrtc::webrtc_common::WebRTCError,
    },
    utils::uuid::UUID,
};
use log::{debug, error, info, warn};
use matchbox_socket::{PeerId, PeerState, RtcIceServerConfig, WebRtcSocket};
use tokio::spawn;
use url::Url;

pub struct WebRTCClientInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    pub com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    socket: Option<Arc<Mutex<WebRtcSocket>>>,
    pub peer_socket_map: Arc<Mutex<HashMap<PeerId, ComInterfaceSocketUUID>>>,
    ice_server_config: RtcIceServerConfig,
}
impl MultipleSocketProvider for WebRTCClientInterface {
    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.com_interface_sockets.clone()
    }
}

impl WebRTCClientInterface {
    const RECONNECT_ATTEMPTS: u16 = 3;
    const CHANNEL_ID: usize = 0;
    pub async fn open_reliable(
        address: &str,
        ice_server_config: Option<RtcIceServerConfig>,
    ) -> Result<WebRTCClientInterface, WebRTCError> {
        Self::open(address, ice_server_config, true).await
    }
    pub async fn open_unreliable(
        address: &str,
        ice_server_config: Option<RtcIceServerConfig>,
    ) -> Result<WebRTCClientInterface, WebRTCError> {
        Self::open(address, ice_server_config, false).await
    }
    async fn open(
        address: &str,
        ice_server_config: Option<RtcIceServerConfig>,
        use_reliable_connection: bool,
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
            peer_socket_map: Arc::new(Mutex::new(HashMap::new())),
            ice_server_config: ice_server_config
                .unwrap_or_else(|| RtcIceServerConfig::default()),
        };
        interface.start(use_reliable_connection).await?;
        warn!("done open");

        Ok(interface)
    }
    async fn start(
        &mut self,
        use_reliable_connection: bool,
    ) -> Result<(), WebRTCError> {
        let address = self.address.clone();
        info!("Connecting to WebRTC server at {}", address.to_string());
        let ice_config = self.ice_server_config.clone();
        let (socket, future) = if use_reliable_connection {
            WebRtcSocket::builder(address)
                .reconnect_attempts(Some(Self::RECONNECT_ATTEMPTS))
                .add_reliable_channel()
                .ice_server(ice_config)
                .build()
        } else {
            WebRtcSocket::builder(address)
                .reconnect_attempts(Some(Self::RECONNECT_ATTEMPTS))
                .add_unreliable_channel()
                .ice_server(ice_config)
                .build()
        };

        info!("Connected to WebRTC server");
        let socket = Arc::new(Mutex::new(socket));
        self.socket = Some(socket.clone());
        let interface_uuid = self.uuid.clone();
        let com_interface_sockets = self.com_interface_sockets.clone();
        let peer_socket_map = self.peer_socket_map.clone();
        spawn(async move {
            let rtc_socket = socket.as_ref();
            loop {
                for (peer, state) in rtc_socket.lock().unwrap().update_peers() {
                    let mut peer_socket_map = peer_socket_map.lock().unwrap();
                    let mut com_interface_sockets =
                        com_interface_sockets.lock().unwrap();
                    info!("got state update: {peer:?} {state:?}");
                    match state {
                        PeerState::Connected => {
                            let socket = ComInterfaceSocket::new(
                                interface_uuid.clone(),
                                InterfaceDirection::IN_OUT,
                                1,
                            );
                            let socket_uuid = socket.uuid.clone();
                            com_interface_sockets
                                .add_socket(Arc::new(Mutex::new(socket)));
                            info!("Socket joined: {socket_uuid}");
                            peer_socket_map.insert(peer, socket_uuid);
                        }
                        PeerState::Disconnected => {
                            let socket_uuid =
                                peer_socket_map.get(&peer).unwrap();
                            info!("Socket disconnected: {socket_uuid}");

                            com_interface_sockets.remove_socket(socket_uuid);
                            peer_socket_map.remove(&peer);
                        }
                    }
                }
                return;

                for (peer, packet) in rtc_socket
                    .lock()
                    .unwrap()
                    .channel_mut(Self::CHANNEL_ID)
                    .receive()
                {
                    let peer_socket_map = peer_socket_map.lock().unwrap();
                    let socket_uuid = peer_socket_map.get(&peer).unwrap();

                    let sockets = com_interface_sockets.lock().unwrap();
                    let socket =
                        sockets.get_socket_by_uuid(socket_uuid).unwrap();
                    let socket = socket.lock().unwrap();
                    let receive_queue = socket.receive_queue.clone();
                    let mut queue = receive_queue.lock().unwrap();
                    let message = String::from_utf8_lossy(&packet);
                    debug!("Message from {socket_uuid}: {message:?}");

                    queue.extend(packet);
                    drop(queue);
                    drop(socket);
                }
            }
        });
        spawn(async move {
            future
                .await
                .map_err(|_| {
                    error!("Failed to connect to WebRTC server");
                    WebRTCError::ConnectionError
                })
                .unwrap_or_else(|_| {
                    error!("Failed to connect to WebRTC server");
                });
        });
        Ok(())
    }
}

impl ComInterface for WebRTCClientInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let peer_socket_map = self.peer_socket_map.clone();
        let rtc_socket = self.socket.clone();
        if rtc_socket.is_none() {
            error!("Client is not connected");
            return Box::pin(async { false });
        }
        warn!("sendblock");
        let peer_id = {
            let peer_socket_map = peer_socket_map.lock().unwrap();
            peer_socket_map
                .iter()
                .find(|(_, uuid)| *uuid == &socket_uuid)
                .map(|(peer, _)| *peer)
        };

        if peer_id.is_none() {
            error!("Peer not found");
            return Box::pin(async { false });
        }

        let rtc_socket = rtc_socket.unwrap();
        Box::pin(async move {
            debug!("Sending block: {:?}", block);
            rtc_socket
                .lock()
                .unwrap()
                .channel_mut(Self::CHANNEL_ID)
                .try_send(block.into(), peer_id.unwrap())
                .map_err(|e| {
                    error!("Error sending message: {:?}", e);
                    false
                })
                .is_ok()
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
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

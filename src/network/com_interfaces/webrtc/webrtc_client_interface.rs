use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::network::com_interfaces::com_interface::ComInterfaceState;
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
        webrtc::webrtc_common::WebRTCError,
    },
    utils::uuid::UUID,
};
use futures::{select, FutureExt};
use futures_timer::Delay;
use log::{debug, error, info, warn};
use matchbox_socket::{PeerId, PeerState, RtcIceServerConfig, WebRtcSocket};
use tokio::spawn;
use url::Url;

pub struct WebRTCClientInterface {
    pub address: Url,
    socket: Option<Arc<Mutex<WebRtcSocket>>>,
    pub peer_socket_map: Arc<Mutex<HashMap<PeerId, ComInterfaceSocketUUID>>>,
    ice_server_config: RtcIceServerConfig,
    info: ComInterfaceInfo,
}
impl MultipleSocketProvider for WebRTCClientInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        return self.get_sockets();
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
        let address =
            Url::parse(address).map_err(|_| WebRTCError::InvalidURL)?;

        let mut interface = WebRTCClientInterface {
            address,
            socket: None,
            peer_socket_map: Arc::new(Mutex::new(HashMap::new())),
            ice_server_config: ice_server_config
                .unwrap_or_else(|| RtcIceServerConfig::default()),
            info: ComInterfaceInfo::new(),
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
        let interface_uuid = self.get_uuid().clone();
        let com_interface_sockets = self.get_sockets().clone();
        let peer_socket_map = self.peer_socket_map.clone();
        let loop_fut = future.fuse();

        let state = self.get_info().get_state();
        spawn(async move {
            futures::pin_mut!(loop_fut);
            let timeout = Delay::new(Duration::from_millis(100));
            futures::pin_mut!(timeout);
            let mut timeout = timeout;
            state
                .lock()
                .unwrap()
                .set_state(ComInterfaceState::Connecting);
            let mut is_connected = false;
            let rtc_socket = socket.as_ref();
            loop {
                for (peer, peer_state) in
                    rtc_socket.lock().unwrap().update_peers()
                {
                    let mut peer_socket_map = peer_socket_map.lock().unwrap();
                    let mut com_interface_sockets =
                        com_interface_sockets.lock().unwrap();
                    if !is_connected {
                        state
                            .lock()
                            .unwrap()
                            .set_state(ComInterfaceState::Connected);
                        is_connected = true;
                    }
                    match peer_state {
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
                select! {
                    _ = (&mut timeout).fuse() => {
                        timeout.reset(Duration::from_millis(100));
                    }
                    // Break if the message loop ends (disconnected, closed, etc.)
                    _ = &mut loop_fut => {
                        break;
                    }
                }
            }
            state.lock().unwrap().set_state(ComInterfaceState::Closed);
            warn!("WebRTC socket closed");
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

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "webrtc".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }

    delegate_com_interface_info!();
}

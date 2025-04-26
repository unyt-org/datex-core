use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::network::com_interfaces::com_interface::ComInterfaceState;
use crate::task::spawn;
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
use url::Url;

use super::webrtc_common::WebRTCError;

pub struct WebRTCClientInterface {
    pub address: Url,
    websocket: Option<Arc<Mutex<WebRtcSocket>>>,
    pub peer_socket_map: Arc<Mutex<HashMap<PeerId, ComInterfaceSocketUUID>>>,
    ice_server_config: RtcIceServerConfig,
    info: ComInterfaceInfo,
}
impl MultipleSocketProvider for WebRTCClientInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
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
        let address =
            Url::parse(address).map_err(|_| WebRTCError::InvalidURL)?;

        let mut interface = WebRTCClientInterface {
            address,
            websocket: None,
            peer_socket_map: Arc::new(Mutex::new(HashMap::new())),
            ice_server_config: ice_server_config.unwrap_or_default(),
            info: ComInterfaceInfo::new(),
        };
        interface.start(use_reliable_connection).await?;
        Ok(interface)
    }

    async fn start(
        &mut self,
        use_reliable_connection: bool,
    ) -> Result<(), WebRTCError> {
        let address = self.address.clone();
        info!("Connecting to WebRTC server at {address}");
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
        self.websocket = Some(socket.clone());
        let interface_uuid = self.get_uuid().clone();
        let com_interface_sockets = self.get_sockets().clone();
        let peer_socket_map = self.peer_socket_map.clone();
        let loop_fut = future.fuse();

        let state = self.get_info().state.clone();
        spawn(async move {
            futures::pin_mut!(loop_fut);
            let timeout = Delay::new(Duration::from_millis(100));
            futures::pin_mut!(timeout);
            let mut timeout = timeout;
            let rtc_socket = socket.as_ref();
            let mut is_connected = false;
            loop {
                let id = socket.lock().unwrap().id();
                if !is_connected && id.is_some() {
                    state.lock().unwrap().set(ComInterfaceState::Connected);
                    is_connected = true;
                }

                for (peer, peer_state) in
                    rtc_socket.lock().unwrap().update_peers()
                {
                    let mut peer_socket_map = peer_socket_map.lock().unwrap();
                    let mut com_interface_sockets =
                        com_interface_sockets.lock().unwrap();
                    match peer_state {
                        PeerState::Connected => {
                            let socket = ComInterfaceSocket::new(
                                interface_uuid.clone(),
                                InterfaceDirection::InOut,
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
            state.lock().unwrap().set(ComInterfaceState::Destroyed);
            warn!("WebRTC socket closed");
        });
        let peer_socket_map = self.peer_socket_map.clone();
        let timeout = Delay::new(Duration::from_millis(1000));
        futures::pin_mut!(timeout);
        loop {
            let state = self.get_state();
            if state == ComInterfaceState::Connected {
                break;
            }
            if state == ComInterfaceState::Destroyed {
                return Err(WebRTCError::ConnectionError);
            }
            select! {
                _ = (&mut timeout).fuse() => {
                    timeout.reset(Duration::from_millis(1000));
                }
            }
        }
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
        let rtc_socket = self.websocket.clone();
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
            debug!("Sending block: {block:?}");
            rtc_socket
                .lock()
                .unwrap()
                .channel_mut(Self::CHANNEL_ID)
                .try_send(block.into(), peer_id.unwrap())
                .map_err(|e| {
                    error!("Error sending message: {e:?}");
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
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let websocket = self.websocket.clone();
        Box::pin(async move {
            if websocket.is_some() {
                let websocket: Arc<Mutex<WebRtcSocket>> = websocket.unwrap();
                let mut websocket = websocket.lock().unwrap();
                websocket.close();
            }
            true
        })
    }
    delegate_com_interface_info!();
}

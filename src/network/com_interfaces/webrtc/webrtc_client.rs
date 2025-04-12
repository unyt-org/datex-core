use std::{
    cell::RefCell,
    collections::HashMap,
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
use matchbox_socket::{PeerId, PeerState, WebRtcChannel, WebRtcSocket};
use tokio::spawn;
use url::Url;

pub struct WebRTCClientInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    pub com_interface_sockets: Arc<Mutex<ComInterfaceSockets>>,
    socket: Option<Arc<Mutex<WebRtcSocket>>>,
    pub peer_socket_map: Arc<Mutex<HashMap<PeerId, ComInterfaceSocketUUID>>>,
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
            peer_socket_map: Arc::new(Mutex::new(HashMap::new())),
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
        let com_interface_sockets = self.com_interface_sockets.clone();
        let peer_socket_map = self.peer_socket_map.clone();
        spawn(async move {
            let rtc_socket = socket.as_ref();
            let mut rtc_socket = rtc_socket.lock().unwrap();
            loop {
                for (peer, state) in rtc_socket.update_peers() {
                    match state {
                        PeerState::Connected => {
                            let socket = ComInterfaceSocket::new(
                                interface_uuid.clone(),
                                InterfaceDirection::IN_OUT,
                                1,
                            );
                            let socket_uuid = socket.uuid.clone();
                            com_interface_sockets
                                .lock()
                                .unwrap()
                                .add_socket(Arc::new(Mutex::new(socket)));

                            peer_socket_map
                                .lock()
                                .unwrap()
                                .insert(peer, socket_uuid);
                            info!("Peer joined: {peer}");
                            // let packet = "hello friend!"
                            //     .as_bytes()
                            //     .to_vec()
                            //     .into_boxed_slice();
                            // socket.channel_mut(CHANNEL_ID).send(packet, peer);
                        }
                        PeerState::Disconnected => {
                            info!("Peer left: {peer}");
                        }
                    }
                }

                for (peer, packet) in
                    rtc_socket.channel_mut(Self::CHANNEL_ID).receive()
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
        let com_interface_sockets = self.com_interface_sockets.clone();
        // let socket = com_interface_sockets
        //     .lock()
        //     .unwrap()
        //     .get_socket_by_uuid(&socket)
        //     .unwrap();
        let rtc_socket = self.socket.clone();
        if rtc_socket.is_none() {
            error!("Client is not connected");
            return Box::pin(async { false });
        }
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
                .send(block.into(), peer_id.unwrap());
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

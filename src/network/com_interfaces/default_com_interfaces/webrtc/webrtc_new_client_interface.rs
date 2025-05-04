use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    hash::Hash,
    io::Error,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use datex_macros::{com_interface, create_opener};
use log::{debug, info};
use serde::{de::DeserializeOwned, Serialize};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::MediaEngine, APIBuilder,
    },
    data_channel::{
        self, data_channel_init::RTCDataChannelInit,
        data_channel_message::DataChannelMessage, OnOpenHdlrFn, RTCDataChannel,
    },
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_gatherer::OnLocalCandidateHdlrFn,
        ice_server::RTCIceServer,
    },
    interceptor::registry::Registry,
    mdns::message::name,
    mux::endpoint,
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
    sdp::description,
    turn::proto::data,
    util::vnet::interface,
};

use crate::{
    datex_values::Endpoint,
    network::com_interfaces::{
        com_interface::ComInterfaceState,
        com_interface_properties::InterfaceDirection,
        com_interface_socket::ComInterfaceSocket,
        socket_provider::SingleSocketProvider,
    },
};
use crate::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceInfo, ComInterfaceSockets,
            ComInterfaceUUID,
        },
        com_interface_properties::InterfaceProperties,
        com_interface_socket::ComInterfaceSocketUUID,
        socket_provider::MultipleSocketProvider,
    },
    set_opener,
};

use super::webrtc_common::WebRTCError;

pub struct WebRTCNewClientInterface {
    info: ComInterfaceInfo,
    peer_connection: Option<Arc<RTCPeerConnection>>,
    pub remote_endpoint: Endpoint,
    pub ice_candidates: Arc<Mutex<VecDeque<Vec<u8>>>>,
    data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    has_media_support: bool,
    rtc_configuration: RTCConfiguration,
}
impl SingleSocketProvider for WebRTCNewClientInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}

#[com_interface]
impl WebRTCNewClientInterface {
    pub fn new(endpoint: impl Into<Endpoint>) -> WebRTCNewClientInterface {
        let endpoint: Endpoint = endpoint.into();
        let mut interface = WebRTCNewClientInterface {
            info: ComInterfaceInfo::new(),
            peer_connection: None,
            remote_endpoint: endpoint.clone(),
            ice_candidates: Arc::new(Mutex::new(VecDeque::new())),
            data_channel: Arc::new(Mutex::new(None)),
            has_media_support: false,
            rtc_configuration: RTCConfiguration {
                ..Default::default()
            },
        };
        let mut properties = interface.init_properties();
        properties.name = Some(endpoint.to_string());
        interface.info.interface_properties = Some(properties);
        interface
    }

    pub fn set_ice_servers(
        mut self,
        ice_servers: Vec<RTCIceServer>,
    ) -> WebRTCNewClientInterface {
        self.rtc_configuration.ice_servers = ice_servers;
        self
    }

    pub fn new_with_media_support(
        endpoint: impl Into<Endpoint>,
    ) -> WebRTCNewClientInterface {
        let mut interface = WebRTCNewClientInterface::new(endpoint.into());
        interface.has_media_support = true;
        interface
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), WebRTCError> {
        let api = APIBuilder::new();
        let api = if self.has_media_support {
            let mut media_engine = MediaEngine::default();
            media_engine
                .register_default_codecs()
                .map_err(|_| WebRTCError::MediaEngineError)?;
            let mut registry = Registry::new();
            registry =
                register_default_interceptors(registry, &mut media_engine)
                    .map_err(|_| WebRTCError::MediaEngineError)?;
            api.with_media_engine(media_engine)
                .with_interceptor_registry(registry)
        } else {
            api
        }
        .build();

        let peer_connection = Arc::new(
            api.new_peer_connection(self.rtc_configuration.clone())
                .await
                .unwrap(),
        );
        self.peer_connection = Some(peer_connection.clone());
        self.setup_ice_candidate_handler();

        let data_channel_store = self.data_channel.clone();

        // If the peer sends a data channel, we need to handle it.
        let sockets = self.get_sockets();
        peer_connection.on_data_channel(Box::new(move |data_channel| {
            let data_channel = data_channel.clone();
            let sockets = sockets.clone();
            {
                // Only one channel can be handled by the interface.
                let mut lock = data_channel_store.lock().unwrap();
                if lock.is_some() {
                    return Box::pin(async {});
                }
                *lock = Some(data_channel.clone());
            }
            Self::handle_receive(data_channel, sockets);
            Box::pin(async {})
        }));
        Ok(())
    }

    fn handle_receive(
        data_channel: Arc<RTCDataChannel>,
        sockets: Arc<Mutex<ComInterfaceSockets>>,
    ) {
        data_channel.clone().on_message(Box::new(move |msg| {
            let data = msg.data.to_vec();
            let data_channel = data_channel.clone();
            {
                let sockets = sockets.lock().unwrap();
                let socket = sockets.sockets.values().next().unwrap();
                let socket = socket.lock().unwrap();
                let mut receive_queue = socket.receive_queue.lock().unwrap();
                debug!("Received message: {:?}", data);
                receive_queue.extend(data);
            }
            Box::pin(async move {
                // data_channel.send(&Bytes::from("pong")).await.unwrap();
            })
        }));
    }

    /// Creates an offer for the WebRTC connection.
    /// This function sets up a single data channel that is either reliable or unreliable.
    /// The `use_reliable_connection` parameter determines whether the data channel is reliable.
    pub async fn create_offer(
        &mut self,
        use_reliable_connection: bool,
    ) -> Vec<u8> {
        let channel_config = RTCDataChannelInit {
            ordered: Some(use_reliable_connection),
            ..Default::default()
        };
        let offer = self.create_session_description(channel_config).await;
        Self::serialize(&offer).unwrap()
    }

    pub async fn set_remote_description(
        &self,
        description: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        let sdp = Self::deserialize::<RTCSessionDescription>(&description);
        if sdp.is_err() {
            return Err(WebRTCError::InvalidSdp);
        }
        if let Some(peer_connection) = &self.peer_connection
            && sdp.is_ok()
        {
            peer_connection
                .set_remote_description(sdp.unwrap())
                .await
                .unwrap();
            Ok(())
        } else {
            panic!("Peer connection not initialized");
        }
    }

    pub async fn create_answer(&self) -> Vec<u8> {
        if let Some(peer_connection) = &self.peer_connection {
            let answer = peer_connection.create_answer(None).await.unwrap();
            let mut gather_complete =
                peer_connection.gathering_complete_promise().await;

            peer_connection
                .set_local_description(answer.clone())
                .await
                .unwrap();

            let _ = gather_complete.recv().await;
            let description =
                peer_connection.local_description().await.unwrap();
            Self::serialize(&description).unwrap()
        } else {
            panic!("Peer connection not initialized");
        }
    }

    pub async fn add_ice_candidate(
        &mut self,
        candidate: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        let candidate = Self::deserialize::<RTCIceCandidateInit>(&candidate)
            .map_err(|_| WebRTCError::InvalidCandidate)?;
        if let Some(peer_connection) = &self.peer_connection {
            // self.remote_endpoint.clone()
            if self.get_socket().is_some() {
                return Ok(());
            }

            peer_connection.add_ice_candidate(candidate).await.unwrap();
            let socket = ComInterfaceSocket::new(
                self.get_uuid().clone(),
                InterfaceDirection::InOut,
                1,
            );
            let socket_uuid = socket.uuid.clone();
            self.add_socket(Arc::new(Mutex::new(socket)));
            self.register_socket_endpoint(
                socket_uuid,
                self.remote_endpoint.clone(),
                1,
            )
            .unwrap();
            Ok(())
        } else {
            panic!("Peer connection not initialized");
        }
    }

    async fn create_session_description(
        &mut self,
        channel_config: RTCDataChannelInit,
    ) -> RTCSessionDescription {
        if let Some(peer_connection) = &self.peer_connection {
            let data_channel = peer_connection
                .create_data_channel("datex", Some(channel_config))
                .await
                .unwrap();
            let sockets = self.get_sockets();
            self.data_channel = Arc::new(Mutex::new(Some(data_channel)));

            let data_channel = self.data_channel.clone();
            let callback: OnOpenHdlrFn = Box::new(move || {
                let lock = data_channel.clone();
                let data_channel = lock.lock().unwrap();
                let data_channel = data_channel.clone().unwrap();
                Self::handle_receive(data_channel.clone(), sockets.clone());
                Box::pin(async {})
            });

            let data_channel = self.data_channel.clone();
            let data_channel = data_channel.lock().unwrap();
            let data_channel = data_channel.clone().unwrap();
            data_channel.on_open(callback);
            drop(data_channel);

            let offer = peer_connection.create_offer(None).await.unwrap();
            let mut gather_complete =
                peer_connection.gathering_complete_promise().await;
            peer_connection
                .set_local_description(offer.clone())
                .await
                .unwrap();
            let _ = gather_complete.recv().await;
            peer_connection.local_description().await.unwrap()
        } else {
            panic!("Peer connection not initialized");
        }
    }

    fn serialize<T: Serialize>(
        value: &T,
    ) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_string(value).map(|s| s.into_bytes())
    }

    fn deserialize<T: DeserializeOwned>(
        value: &[u8],
    ) -> Result<T, serde_json::Error> {
        let string = std::str::from_utf8(value).unwrap();
        serde_json::from_str(string)
    }

    fn setup_ice_candidate_handler(&mut self) {
        let properties = self.get_properties();
        let name = properties.name.as_ref().unwrap_or(&"".to_string()).clone();
        if let Some(peer_connection) = &self.peer_connection {
            let candidates = self.ice_candidates.clone();
            peer_connection.on_ice_candidate(Box::new(
                move |candidate: Option<RTCIceCandidate>| {
                    if let Some(candidate) = candidate {
                        let candidate_init = candidate.to_json();
                        if let Ok(candidate) = &candidate_init {
                            let mut candidates = candidates.lock().unwrap();
                            candidates.push_back(
                                Self::serialize(&candidate).unwrap(),
                            );
                        }
                    }
                    Box::pin(async {})
                },
            ));
        } else {
            panic!("Peer connection not initialized");
        }
    }
}

impl ComInterface for WebRTCNewClientInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let channel_guard = self.data_channel.lock().unwrap();
        if let Some(data_channel) = channel_guard.clone() {
            Box::pin(async move {
                let bytes = Bytes::from(block.to_vec());
                data_channel.send(&bytes).await.is_ok()
            })
        } else {
            Box::pin(async move { false })
        }
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            interface_type: "webrtc".to_string(),
            channel: "webrtc".to_string(),
            round_trip_time: Duration::from_millis(40),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move { true })
    }
    delegate_com_interface_info!();
    set_opener!(open);
}

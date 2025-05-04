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
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    data_channel::{
        data_channel_init::RTCDataChannelInit,
        data_channel_message::DataChannelMessage, RTCDataChannel,
    },
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_gatherer::OnLocalCandidateHdlrFn,
    },
    mdns::message::name,
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
    turn::proto::data,
};

use crate::{
    datex_values::Endpoint,
    network::com_interfaces::{
        com_interface::ComInterfaceState,
        com_interface_properties::InterfaceDirection,
        com_interface_socket::ComInterfaceSocket,
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
    pub ice_candidates: Arc<Mutex<VecDeque<RTCIceCandidateInit>>>,
    data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
}
impl MultipleSocketProvider for WebRTCNewClientInterface {
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
        };
        let mut properties = interface.init_properties();
        properties.name = Some(endpoint.to_string());
        interface.info.interface_properties = Some(properties);
        interface
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), WebRTCError> {
        let api = APIBuilder::new().build();
        let peer_connection = Arc::new(
            api.new_peer_connection(Default::default()).await.unwrap(),
        );
        self.peer_connection = Some(peer_connection.clone());
        self.setup_ice_candidate_handler();

        let data_channel_store = self.data_channel.clone();

        peer_connection.on_data_channel(Box::new(move |dc| {
            let data_channel = dc.clone();
            let name = data_channel.label();
            {
                let mut lock = data_channel_store.lock().unwrap();
                *lock = Some(data_channel.clone());
            }
            data_channel.on_message(Box::new(move |msg| {
                let data = msg.data;
                debug!("Received message: {:?}", data);
                Box::pin(async {})
            }));
            Box::pin(async {})
        }));
        Ok(())
    }

    pub async fn create_offer(&mut self) -> RTCSessionDescription {
        if let Some(peer_connection) = &self.peer_connection {
            let channel_config = RTCDataChannelInit {
                ordered: Some(true),
                ..Default::default()
            };
            let data_channel = peer_connection
                .create_data_channel("datex", Some(channel_config))
                .await
                .unwrap();
            self.data_channel = Arc::new(Mutex::new(Some(data_channel)));
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

    pub async fn set_remote_description(&self, sdp: RTCSessionDescription) {
        if let Some(peer_connection) = &self.peer_connection {
            peer_connection.set_remote_description(sdp).await.unwrap();
        } else {
            panic!("Peer connection not initialized");
        }
    }

    pub async fn create_answer(&self) -> RTCSessionDescription {
        if let Some(peer_connection) = &self.peer_connection {
            let answer = peer_connection.create_answer(None).await.unwrap();
            let mut gather_complete =
                peer_connection.gathering_complete_promise().await;

            peer_connection
                .set_local_description(answer.clone())
                .await
                .unwrap();

            let _ = gather_complete.recv().await;
            peer_connection.local_description().await.unwrap()
        } else {
            panic!("Peer connection not initialized");
        }
    }

    pub async fn add_ice_candidate(&mut self, candidate: RTCIceCandidateInit) {
        if let Some(peer_connection) = &self.peer_connection {
            if self
                .get_socket_uuid_for_endpoint(self.remote_endpoint.clone())
                .is_some()
            {
                return;
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
        }
    }

    fn setup_ice_candidate_handler(&mut self) {
        let properties = self.get_properties();
        let name = properties.name.as_ref().unwrap_or(&"".to_string()).clone();
        if let Some(peer_connection) = &self.peer_connection {
            let candidates = self.ice_candidates.clone();
            peer_connection.on_ice_candidate(Box::new(
                move |candidate: Option<RTCIceCandidate>| {
                    if let Some(candidate) = candidate {
                        let candidate_init = candidate.to_json().unwrap();
                        // info!(
                        //     "{}: New ICE candidate: {:?}",
                        //     name, candidate.port
                        // );
                        let mut candidates = candidates.lock().unwrap();
                        candidates.push_back(candidate_init);
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

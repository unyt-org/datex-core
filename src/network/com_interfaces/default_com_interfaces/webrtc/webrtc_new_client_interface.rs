use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use datex_macros::{com_interface, create_opener};
use log::{debug, info};
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    ice_transport::ice_candidate::RTCIceCandidate,
    peer_connection::{configuration::RTCConfiguration, RTCPeerConnection},
};

use crate::network::com_interfaces::com_interface::ComInterfaceState;
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
}
impl MultipleSocketProvider for WebRTCNewClientInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}

#[com_interface]
impl WebRTCNewClientInterface {
    pub fn new(name: &str) -> WebRTCNewClientInterface {
        let info = ComInterfaceInfo::new();
        WebRTCNewClientInterface { info }
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), WebRTCError> {
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs().unwrap();

        let api = APIBuilder::new().with_media_engine(media_engine).build();
        let config = RTCConfiguration::default(); // FIXME allow custom config
        let peer_connection =
            Arc::new(api.new_peer_connection(config).await.unwrap());
        self.setup_ice_candidates(peer_connection.clone()).await;
        self.setup_tracks(peer_connection.clone()).await;
        peer_connection.on_data_channel(Box::new(move |data_channel| {
            data_channel.on_message(Box::new(|msg| {
                info!("New message on data channel: {:?}", msg);
                // Handle the message
                Box::pin(async {})
            }));
            Box::pin(async {})
        }));
        Ok(())
    }
    async fn setup_ice_candidates(
        &self,
        peer_connection: Arc<RTCPeerConnection>,
    ) {
        peer_connection.on_ice_candidate(Box::new(
            |candidate: Option<RTCIceCandidate>| {
                if let Some(candidate) = candidate {
                    info!("New ICE candidate: {:?}", candidate);
                    // Send the candidate to the remote peer through signaling server
                }
                Box::pin(async {})
            },
        ));
    }
    async fn setup_tracks(&self, peer_connection: Arc<RTCPeerConnection>) {
        peer_connection.on_track(Box::new(|track, _, _| {
            info!("New track received: {:?}", track);
            Box::pin(async {})
        }));
    }
}

impl ComInterface for WebRTCNewClientInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(async move {
            debug!("Sending block: {block:?}");
            true
        })
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

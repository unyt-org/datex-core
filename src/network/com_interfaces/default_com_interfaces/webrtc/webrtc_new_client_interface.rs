use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    io::Error,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use datex_macros::{com_interface, create_opener};
use log::{debug, info};
use tokio::sync::Notify;
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_gatherer::OnLocalCandidateHdlrFn,
        ice_gatherer_state::RTCIceGathererState,
        ice_gathering_state::RTCIceGatheringState,
    },
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
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
    peer_connection: Option<Arc<RTCPeerConnection>>,
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
        WebRTCNewClientInterface {
            info,
            peer_connection: None,
        }
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), WebRTCError> {
        let api = APIBuilder::new().build();
        let peer_connection = Arc::new(
            api.new_peer_connection(Default::default()).await.unwrap(),
        );
        self.peer_connection = Some(peer_connection.clone());
        Ok(())
    }

    pub async fn create_offer(&self) -> RTCSessionDescription {
        if let Some(peer_connection) = &self.peer_connection {
            let notify = Arc::new(Notify::new());
            let notify_clone = notify.clone();

            peer_connection.on_ice_gathering_state_change(Box::new(
                move |state| {
                    if state == RTCIceGathererState::Complete {
                        notify_clone.notify_one();
                    }
                    Box::pin(async {})
                },
            ));

            let offer = peer_connection.create_offer(None).await.unwrap();
            peer_connection.set_local_description(offer).await.unwrap();

            // ✅ Wait for ICE gathering to be complete
            notify.notified().await;

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

    pub async fn add_ice_candidate(&self, candidate: RTCIceCandidateInit) {
        if let Some(peer_connection) = &self.peer_connection {
            peer_connection.add_ice_candidate(candidate).await.unwrap();
        }
    }

    pub fn on_ice_candidate(&self, f: OnLocalCandidateHdlrFn) {
        if let Some(peer_connection) = &self.peer_connection {
            peer_connection.on_ice_candidate(f);
        } else {
            panic!("Peer connection not initialized");
        }
    }

    // pub async fn set_offer(
    //     &mut self,
    //     id: &str,
    //     offer: RTCSessionDescription,
    // ) -> Result<(), WebRTCError> {
    //     let peer_connection = self
    //         .peer_connections
    //         .get(id)
    //         .ok_or(WebRTCError::InvalidURL)?;

    //     peer_connection.set_remote_description(offer).await.unwrap();
    //     // let answer = peer_connection.create_answer(None).await.unwrap();

    //     // let mut gather_complete =
    //     //     peer_connection.gathering_complete_promise().await;

    //     // peer_connection.set_local_description(answer).await.unwrap();
    //     // let _ = gather_complete.recv().await;
    //     // let local_desc = peer_connection.local_description().await.unwrap();
    //     // info!("Local description: {:?}", local_desc);
    //     Ok(())
    // }

    // pub async fn create_offer(&mut self, id: &str) -> RTCSessionDescription {
    //     let mut media_engine = MediaEngine::default();
    //     media_engine.register_default_codecs().unwrap();

    //     let api = APIBuilder::new() /*.with_media_engine(media_engine) */
    //         .build();
    //     let config = RTCConfiguration::default(); // FIXME allow custom config
    //     let peer_connection =
    //         Arc::new(api.new_peer_connection(config).await.unwrap());
    //     let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    //     self.setup_ice_candidates(peer_connection.clone()).await;
    //     // self.setup_tracks(peer_connection.clone()).await;
    //     peer_connection.on_peer_connection_state_change(Box::new(
    //         move |s: RTCPeerConnectionState| {
    //             info!("Peer connection state changed: {:?}", s);
    //             match s {
    //                 RTCPeerConnectionState::Connected => {
    //                     info!("Peer connection is connected");

    //                     let _ = done_tx.try_send(());
    //                 }
    //                 RTCPeerConnectionState::Disconnected => {
    //                     info!("Peer connection is disconnected");

    //                     let _ = done_tx.try_send(());
    //                 }
    //                 RTCPeerConnectionState::Failed => {
    //                     info!("Peer connection failed");
    //                     let _ = done_tx.try_send(());
    //                 }
    //                 _ => {}
    //             }
    //             Box::pin(async {})
    //         },
    //     ));

    //     peer_connection
    //         .on_data_channel(Box::new(move |channel: Arc<RTCDataChannel>| {
    //             let channel_label = channel.label().to_owned();
    //             let channel_id = channel.id();
    //             println!("New DataChannel {} {}", channel_label, channel_id);

    //             // Register channel opening handling
    //             Box::pin(async move {
    //                 let channel_clone = Arc::clone(&channel);
    //                 let channel_label_clone = channel_label.clone();
    //                 let channel_id = channel_id.clone();
    //                 channel.on_open(Box::new(move || {
    //                     info!("Data channel '{}'-'{}' open.", channel_label_clone, channel_id);
    //                     Box::pin(async move {
    //                         let mut result = Result::<usize, webrtc::Error>::Ok(0);
    //                         while result.is_ok() {
    //                             let timeout = tokio::time::sleep(Duration::from_secs(5));
    //                             tokio::pin!(timeout);

    //                             tokio::select! {
    //                                 _ = timeout.as_mut() =>{
    //                                     result = channel_clone.send_text("hello").await.map_err(Into::into);
    //                                 }
    //                             };
    //                         }
    //                     })
    //                 }));

    //                 // Register text message handling
    //                 channel.on_message(Box::new(move |msg: DataChannelMessage| {
    //                     let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
    //                     println!("Message from DataChannel '{}': '{}'", channel_label, msg_str);
    //                     Box::pin(async {})
    //                 }));
    //             })
    //         }));
    //     self.peer_connections
    //         .insert(id.to_string(), peer_connection.clone());
    //     let mut offer = peer_connection.create_offer(None).await.unwrap();
    //     spawn(async move {
    //         done_rx.recv().await;
    //     });

    //     let mut gather_complete =
    //         peer_connection.gathering_complete_promise().await;

    //     peer_connection.set_local_description(offer).await.unwrap();
    //     let _ = gather_complete.recv().await;

    //     if let Some(local_desc) = peer_connection.local_description().await {
    //         info!("Local description: {:?}", local_desc);
    //         return local_desc;
    //     } else {
    //         panic!("Failed to get local description");
    //     }
    // }

    // async fn setup_ice_candidates(
    //     &self,
    //     peer_connection: Arc<RTCPeerConnection>,
    // ) {
    //     peer_connection.on_ice_candidate(Box::new(
    //         |candidate: Option<RTCIceCandidate>| {
    //             if let Some(candidate) = candidate {
    //                 info!("New ICE candidate: {:?}", candidate);
    //                 // Send the candidate to the remote peer through signaling server
    //             }
    //             Box::pin(async {})
    //         },
    //     ));
    // }
    // async fn setup_tracks(&self, peer_connection: Arc<RTCPeerConnection>) {
    //     peer_connection.on_track(Box::new(|track, _, _| {
    //         info!("New track received: {:?}", track);
    //         Box::pin(async {})
    //     }));
    // }
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

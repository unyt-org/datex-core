use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    datex_values::Endpoint,
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceInfo, ComInterfaceSockets,
            ComInterfaceState,
        },
        com_interface_properties::InterfaceProperties,
        com_interface_socket::ComInterfaceSocketUUID,
        default_com_interfaces::webrtc::webrtc_common_new::structures::RTCSdpTypeDX,
        socket_provider::SingleSocketProvider,
    },
    set_opener,
    task::spawn_local,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{channel::mpsc, SinkExt, StreamExt, TryFutureExt};

use super::webrtc_common_new::{
    data_channels::{DataChannel, DataChannels},
    structures::{
        RTCIceCandidateInitDX, RTCIceServer, RTCSessionDescriptionDX,
    },
    utils::WebRTCError,
    webrtc_commons::WebRTCCommon,
    webrtc_trait::{WebRTCTrait, WebRTCTraitInternal},
};
use datex_macros::{com_interface, create_opener};
use log::{error, info};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::MediaEngine, APIBuilder,
    },
    data_channel::{
        data_channel_init::RTCDataChannelInit, OnOpenHdlrFn, RTCDataChannel,
    },
    ice_transport::ice_candidate::{
        RTCIceCandidate, RTCIceCandidateInit,
    },
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};
pub struct WebRTCNativeInterface {
    info: ComInterfaceInfo,
    commons: Arc<Mutex<WebRTCCommon>>,
    peer_connection: Option<Arc<RTCPeerConnection>>,
    data_channels: Arc<Mutex<DataChannels<Arc<RTCDataChannel>>>>,
    rtc_configuration: RTCConfiguration,
}
impl SingleSocketProvider for WebRTCNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}
impl WebRTCTrait<Arc<RTCDataChannel>> for WebRTCNativeInterface {
    fn new(peer_endpoint: impl Into<Endpoint>) -> Self {
        let commons = WebRTCCommon::new(peer_endpoint);
        WebRTCNativeInterface {
            info: ComInterfaceInfo::default(),
            commons: Arc::new(Mutex::new(commons)),
            peer_connection: None,
            data_channels: Arc::new(Mutex::new(DataChannels::new())),
            rtc_configuration: RTCConfiguration {
                ..Default::default()
            },
        }
    }
    fn new_with_ice_servers(
        peer_endpoint: impl Into<Endpoint>,
        ice_servers: Vec<RTCIceServer>,
    ) -> Self {
        let interface = Self::new(peer_endpoint);
        interface.set_ice_servers(ice_servers);
        interface
    }
}

#[async_trait(?Send)]
impl WebRTCTraitInternal<Arc<RTCDataChannel>> for WebRTCNativeInterface {
    fn provide_data_channels(
        &self,
    ) -> Arc<Mutex<DataChannels<Arc<RTCDataChannel>>>> {
        self.data_channels.clone()
    }
    fn provide_info(&self) -> &ComInterfaceInfo {
        &self.info
    }

    async fn handle_create_data_channel(
        &self,
    ) -> Result<DataChannel<Arc<RTCDataChannel>>, WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            let channel_config = RTCDataChannelInit::default();
            let data_channel = peer_connection
                .create_data_channel("datex", Some(channel_config))
                .await
                .unwrap();
            Ok(DataChannel::new(
                data_channel.label().to_string(),
                data_channel,
            ))
        } else {
            error!("Peer connection is not initialized");
            return Err(WebRTCError::ConnectionError);
        }
    }

    async fn handle_setup_data_channel(
        channel: Arc<Mutex<DataChannel<Arc<RTCDataChannel>>>>,
    ) -> Result<(), WebRTCError> {
        let on_open: OnOpenHdlrFn = Box::new(move || {
            info!("Data channel opened");
            // FIXME TODO handle open
            Box::pin(async {})
        });

        let data_channel = channel.clone();
        let data_channel = data_channel.lock().unwrap();
        data_channel.data_channel.on_open(on_open);
        data_channel.data_channel.on_message(Box::new(move |msg| {
            let data = msg.data.to_vec();
            info!("Received data on data channel: {data:?}");
            Box::pin(async {})
        }));
        Ok(())
    }

    async fn handle_create_offer(
        &self,
    ) -> Result<RTCSessionDescriptionDX, WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            let offer = peer_connection.create_offer(None).await.unwrap();
            // let mut gather_complete =
            //     peer_connection.gathering_complete_promise().await;
            // let _ = gather_complete.recv().await;
            Ok(RTCSessionDescriptionDX {
                sdp_type: RTCSdpTypeDX::Offer,
                sdp: offer.sdp,
            })
        } else {
            error!("Peer connection is not initialized");
            return Err(WebRTCError::ConnectionError);
        }
    }

    async fn handle_add_ice_candidate(
        &self,
        candidate_init: RTCIceCandidateInitDX,
    ) -> Result<(), WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            let ice_candidate = RTCIceCandidateInit {
                candidate: candidate_init.candidate,
                sdp_mid: candidate_init.sdp_mid,
                sdp_mline_index: candidate_init.sdp_mline_index,
                username_fragment: candidate_init.username_fragment,
            };

            peer_connection
                .add_ice_candidate(ice_candidate)
                .await
                .map_err(|e| {
                    error!("Failed to add ICE candidate {e:?}");
                    WebRTCError::InvalidCandidate
                })?;
            Ok(())
        } else {
            error!("Peer connection is not initialized");
            Err(WebRTCError::ConnectionError)
        }
    }

    async fn handle_set_local_description(
        &self,
        description: RTCSessionDescriptionDX,
    ) -> Result<(), WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            let rtc_session_description = {
                if description.sdp_type == RTCSdpTypeDX::Offer {
                    RTCSessionDescription::offer(description.sdp)
                } else if description.sdp_type == RTCSdpTypeDX::Answer {
                    RTCSessionDescription::answer(description.sdp)
                } else {
                    return Err(WebRTCError::InvalidSdp);
                }
            }
            .map_err(|_| WebRTCError::InvalidSdp)?;

            peer_connection
                .set_local_description(rtc_session_description)
                .await
                .map_err(|_| WebRTCError::InvalidSdp)?;
            Ok(())
        } else {
            error!("Peer connection is not initialized");
            return Err(WebRTCError::ConnectionError);
        }
    }

    async fn handle_set_remote_description(
        &self,
        description: RTCSessionDescriptionDX,
    ) -> Result<(), WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            let rtc_session_description = match description.sdp_type {
                RTCSdpTypeDX::Offer => {
                    RTCSessionDescription::offer(description.sdp)
                }
                RTCSdpTypeDX::Answer => {
                    RTCSessionDescription::answer(description.sdp)
                }
                RTCSdpTypeDX::Unspecified => {
                    return Err(WebRTCError::InvalidSdp);
                }
            }
            .map_err(|_| WebRTCError::InvalidSdp)?;

            peer_connection
                .set_remote_description(rtc_session_description)
                .await
                .map_err(|_| WebRTCError::InvalidSdp)?;
            Ok(())
        } else {
            error!("Peer connection is not initialized");
            return Err(WebRTCError::ConnectionError);
        }
    }

    async fn handle_create_answer(
        &self,
    ) -> Result<RTCSessionDescriptionDX, WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            let offer = peer_connection.create_answer(None).await.unwrap();
            Ok(RTCSessionDescriptionDX {
                sdp_type: RTCSdpTypeDX::Answer,
                sdp: offer.sdp,
            })
        } else {
            error!("Peer connection is not initialized");
            return Err(WebRTCError::ConnectionError);
        }
    }

    fn get_commons(&self) -> Arc<Mutex<WebRTCCommon>> {
        self.commons.clone()
    }
}

#[com_interface]
impl WebRTCNativeInterface {
    #[create_opener]
    async fn open(&mut self) -> Result<(), WebRTCError> {
        let has_media_support = true; // TODO
        let api = APIBuilder::new();
        let api = if has_media_support {
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

        {
            // ICE servers
            self.rtc_configuration.ice_servers = self
                .commons
                .lock()
                .unwrap()
                .ice_servers
                .clone()
                .iter()
                .map(|server| webrtc::ice_transport::ice_server::RTCIceServer {
                    urls: server.urls.clone(),
                    username: server.username.clone().unwrap_or("".to_string()),
                    credential: server
                        .credential
                        .clone()
                        .unwrap_or("".to_string()),
                    ..Default::default()
                })
                .collect()
        }
        info!("ICE servers: {:?}", self.rtc_configuration.ice_servers);
        let peer_connection = Arc::new(
            api.new_peer_connection(self.rtc_configuration.clone())
                .await
                .unwrap(),
        );
        self.peer_connection = Some(peer_connection.clone());
        {
            // Data channels
            let data_channels = self.data_channels.clone();
            let (tx_data_channel, mut rx_data_channel) =
                mpsc::unbounded::<Arc<RTCDataChannel>>();
            let tx_clone = tx_data_channel.clone();

            peer_connection.on_data_channel(Box::new(move |data_channel| {
                let mut res = tx_clone.clone();
                let _ = res.start_send(data_channel);
                Box::pin(async {})
            }));
            spawn_local(async move {
                while let Some(channel) = rx_data_channel.next().await {
                    data_channels
                        .clone()
                        .lock()
                        .unwrap()
                        .create_data_channel(
                            channel.label().to_string(),
                            channel.clone(),
                        )
                        .await;
                }
            });
        }
        {
            let commons = self.commons.clone();
            let (tx_ice_candidate, mut rx_ice_candidate) =
                mpsc::unbounded::<RTCIceCandidateInit>();
            let tx_clone = tx_ice_candidate.clone();

            peer_connection.on_ice_candidate(Box::new(
                move |candidate: Option<RTCIceCandidate>| {
                    if let Some(candidate) = candidate {
                        let candidate_init = candidate.to_json();

                        if let Ok(candidate) = &candidate_init {
                            let mut res = tx_clone.clone();
                            let _ = res.start_send(candidate.clone());
                        }
                    }
                    Box::pin(async {})
                },
            ));
            spawn_local(async move {
                while let Some(candidate) = rx_ice_candidate.next().await {
                    commons.clone().lock().unwrap().on_ice_candidate(
                        RTCIceCandidateInitDX {
                            candidate: candidate.candidate,
                            sdp_mid: candidate.sdp_mid,
                            sdp_mline_index: candidate.sdp_mline_index,
                            username_fragment: None,
                        },
                    );
                }
            });
        }
        self.setup_listeners();
        Ok(())
    }
}
impl ComInterface for WebRTCNativeInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        if let Some(channel) =
            self.data_channels.lock().unwrap().get_data_channel("DATEX")
        {
            Box::pin(async move {
                let bytes = Bytes::from(block.to_vec());
                channel
                    .lock()
                    .unwrap()
                    .data_channel
                    .send(&bytes)
                    .await
                    .is_ok()
            })
        } else {
            error!("Failed to send message, data channel not found");
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
        let success = { true };
        Box::pin(async move { success })
    }
    delegate_com_interface_info!();
    set_opener!(open);
}

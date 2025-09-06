use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceError, ComInterfaceFactory,
            ComInterfaceInfo, ComInterfaceSockets, ComInterfaceState,
        },
        com_interface_properties::InterfaceProperties,
        com_interface_socket::ComInterfaceSocketUUID,
        default_com_interfaces::webrtc::webrtc_common::{
            media_tracks::{MediaKind, MediaTrack, MediaTracks},
            structures::RTCSdpTypeDX,
            webrtc_commons::WebRTCInterfaceSetupData,
        },
        socket_provider::SingleSocketProvider,
    },
    set_opener,
    task::spawn_local,
    values::core_values::endpoint::Endpoint,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{StreamExt, channel::mpsc};

use super::webrtc_common::{
    data_channels::{DataChannel, DataChannels},
    structures::{
        RTCIceCandidateInitDX, RTCIceServer, RTCSessionDescriptionDX,
    },
    utils::WebRTCError,
    webrtc_commons::WebRTCCommon,
    webrtc_trait::{WebRTCTrait, WebRTCTraitInternal},
};
use datex_macros::{com_interface, create_opener};
use log::error;
use webrtc::{
    api::{
        APIBuilder,
        interceptor_registry::register_default_interceptors,
        media_engine::{MIME_TYPE_OPUS, MediaEngine},
    },
    data_channel::{
        OnMessageHdlrFn, OnOpenHdlrFn, RTCDataChannel,
        data_channel_init::RTCDataChannelInit,
    },
    ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
    interceptor::registry::Registry,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::{
        RTCRtpEncodingParameters, RTCRtpTransceiverInit,
        rtp_codec::{
            RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
        },
        rtp_transceiver_direction::RTCRtpTransceiverDirection,
    },
    track::{
        track_local::{
            track_local_static_rtp::TrackLocalStaticRTP,
            track_local_static_sample::TrackLocalStaticSample,
        },
        track_remote::{OnMuteHdlrFn, TrackRemote},
    },
};
pub type TrackLocal = dyn webrtc::track::track_local::TrackLocal + Send + Sync;

enum DataChannelEvent {
    Open,
    Message(Vec<u8>),
}

enum MediaChannelEvent {
    Mute,
    Unmute,
}

pub struct WebRTCNativeInterface {
    info: ComInterfaceInfo,
    commons: Arc<Mutex<WebRTCCommon>>,
    peer_connection: Option<Arc<RTCPeerConnection>>,
    data_channels: Rc<RefCell<DataChannels<Arc<RTCDataChannel>>>>,
    remote_media_tracks: Rc<RefCell<MediaTracks<Arc<TrackRemote>>>>,
    local_media_tracks: Rc<RefCell<MediaTracks<Arc<TrackLocal>>>>,
    rtc_configuration: RTCConfiguration,
}
impl SingleSocketProvider for WebRTCNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}
impl WebRTCTrait<Arc<RTCDataChannel>, Arc<TrackRemote>, Arc<TrackLocal>>
    for WebRTCNativeInterface
{
    fn new(peer_endpoint: impl Into<Endpoint>) -> Self {
        let commons = WebRTCCommon::new(peer_endpoint);
        WebRTCNativeInterface {
            info: ComInterfaceInfo::default(),
            commons: Arc::new(Mutex::new(commons)),
            peer_connection: None,
            data_channels: Rc::new(RefCell::new(DataChannels::default())),
            remote_media_tracks: Rc::new(RefCell::new(MediaTracks::default())),
            local_media_tracks: Rc::new(RefCell::new(MediaTracks::default())),
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
impl WebRTCTraitInternal<Arc<RTCDataChannel>, Arc<TrackRemote>, Arc<TrackLocal>>
    for WebRTCNativeInterface
{
    fn provide_data_channels(
        &self,
    ) -> Rc<RefCell<DataChannels<Arc<RTCDataChannel>>>> {
        self.data_channels.clone()
    }

    fn provide_remote_media_tracks(
        &self,
    ) -> Rc<RefCell<MediaTracks<Arc<TrackRemote>>>> {
        self.remote_media_tracks.clone()
    }

    fn provide_local_media_tracks(
        &self,
    ) -> Rc<RefCell<MediaTracks<Arc<TrackLocal>>>> {
        self.local_media_tracks.clone()
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
                .create_data_channel("DATEX", Some(channel_config))
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
        channel: Rc<RefCell<DataChannel<Arc<RTCDataChannel>>>>,
    ) -> Result<(), WebRTCError> {
        let channel_clone = channel.clone();

        let (tx, mut rx) = mpsc::unbounded::<DataChannelEvent>();
        let tx_open = tx.clone();
        let on_open: OnOpenHdlrFn = Box::new(move || {
            let _ = tx_open.unbounded_send(DataChannelEvent::Open);
            Box::pin(async {})
        });

        let tx_msg = tx.clone();
        let on_message: OnMessageHdlrFn = Box::new(move |msg| {
            let data = msg.data.to_vec();
            let _ = tx_msg.unbounded_send(DataChannelEvent::Message(data));
            Box::pin(async {})
        });

        spawn_local(async move {
            let channel_clone = channel_clone.clone();
            while let Some(event) = rx.next().await {
                match event {
                    DataChannelEvent::Open => {
                        if let Some(open_channel) = channel_clone
                            .borrow()
                            .open_channel
                            .borrow()
                            .as_ref()
                        {
                            open_channel();
                        }
                    }
                    DataChannelEvent::Message(data) => {
                        if let Some(on_message) =
                            channel_clone.borrow().on_message.borrow().as_ref()
                        {
                            on_message(data);
                        }
                    }
                }
            }
        });
        let data_channel = channel.clone();
        data_channel.borrow_mut().data_channel.on_open(on_open);
        data_channel
            .borrow_mut()
            .data_channel
            .on_message(on_message);
        Ok(())
    }

    async fn handle_create_media_channel(
        &self,
        id: String,
        kind: MediaKind,
    ) -> Result<MediaTrack<Arc<TrackLocal>>, WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
            // let track = Arc::new(TrackLocalStaticSample::new(
            //     RTCRtpCodecCapability {
            //         mime_type: "audio/opus".to_string(),
            //         ..Default::default()
            //     },
            //     "audio".into(),
            //     "datex".into(),
            // ));

            let track = Arc::new(TrackLocalStaticRTP::new(
                RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_OPUS.to_owned(),
                    ..Default::default()
                },
                id.clone(),
                "datex".to_owned(),
            ));

            // Arc::new(TrackLocalStaticRTP::new(
            //     RTCRtpCodecCapability {
            //         mime_type: "video/VP8".to_string(),
            //         clock_rate: 90000,
            //         channels: 0,
            //         sdp_fmtp_line: "".to_string(),
            //         rtcp_feedback: vec![],
            //     },
            //     "video".to_string(),
            //     "datex".to_string(),
            // ));
            let rtp_sender = peer_connection
                .add_track(track.clone() as Arc<TrackLocal>)
                .await
                .map_err(|e| {
                    error!("Failed to add media track: {e:?}");
                    WebRTCError::ConnectionError
                })?;
            spawn_local(async move {
                let mut rtcp_buf = vec![0u8; 1500];
                while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {
                    println!("Received RTCP packet: {:?}", &rtcp_buf.len());
                }
            });
            println!("Added media track: {:?}", kind);
            Ok(MediaTrack::new(id, kind, track))
        } else {
            error!("Peer connection is not initialized");
            return Err(WebRTCError::ConnectionError);
        }
    }

    async fn handle_setup_media_channel(
        track: Rc<RefCell<MediaTrack<Arc<TrackRemote>>>>,
    ) -> Result<(), WebRTCError> {
        let track_clone = track.clone();
        println!("Setting up media channel: {:?}", track.borrow().kind);

        let (tx, mut rx) = mpsc::unbounded::<MediaChannelEvent>();
        let tx_mute = tx.clone();

        let on_mute: OnMuteHdlrFn = Box::new(move || {
            let _ = tx_mute.unbounded_send(MediaChannelEvent::Mute);
            Box::pin(async {})
        });

        let tx_unmute = tx.clone();
        let on_unmute: OnMuteHdlrFn = Box::new(move || {
            let _ = tx_unmute.unbounded_send(MediaChannelEvent::Unmute);
            Box::pin(async {})
        });

        spawn_local(async move {
            let track_clone = track_clone.clone();
            while let Some(event) = rx.next().await {
                match event {
                    MediaChannelEvent::Mute => {
                        if let Some(open_channel) =
                            track_clone.borrow().on_mute.borrow().as_ref()
                        {
                            open_channel();
                        }
                    }
                    MediaChannelEvent::Unmute => {
                        if let Some(on_unmute) =
                            track_clone.borrow().on_unmute.borrow().as_ref()
                        {
                            on_unmute();
                        }
                    }
                }
            }
        });

        track.borrow().track.onmute(on_mute);
        track.borrow().track.onunmute(on_unmute);

        Ok(())
    }

    async fn handle_create_offer(
        &self,
    ) -> Result<RTCSessionDescriptionDX, WebRTCError> {
        if let Some(peer_connection) = self.peer_connection.as_ref() {
            let offer = peer_connection.create_offer(None).await.unwrap();
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
        let has_media_support = true; // TODO #202
        let api = APIBuilder::new();
        let api = if has_media_support {
            let mut media_engine = MediaEngine::default();
            media_engine
                .register_default_codecs()
                .map_err(|_| WebRTCError::MediaEngineError)?;

            media_engine
                .register_codec(
                    RTCRtpCodecParameters {
                        capability: RTCRtpCodecCapability {
                            mime_type: MIME_TYPE_OPUS.to_owned(),
                            ..Default::default()
                        },
                        payload_type: 120,
                        ..Default::default()
                    },
                    RTPCodecType::Audio,
                )
                .unwrap();

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
            let data_channel_tx_clone = tx_data_channel.clone();

            peer_connection.on_data_channel(Box::new(move |data_channel| {
                print!(
                    "New data channel received: label={:?}\n",
                    data_channel.label()
                );
                let mut res = data_channel_tx_clone.clone();
                let _ = res.start_send(data_channel);
                Box::pin(async {})
            }));

            spawn_local(async move {
                while let Some(channel) = rx_data_channel.next().await {
                    data_channels
                        .clone()
                        .borrow_mut()
                        .create_data_channel(
                            channel.label().to_string(),
                            channel.clone(),
                        )
                        .await;
                }
            });
        }
        {
            // Media tracks
            let media_tracks = self.remote_media_tracks.clone();
            let (tx_media_track, mut rx_media_track) =
                mpsc::unbounded::<Arc<TrackRemote>>();
            let media_track_tx_clone = tx_media_track.clone();

            // peer_connection
            //     .add_transceiver_from_kind(
            //         RTPCodecType::Audio,
            //         Some(RTCRtpTransceiverInit {
            //             direction: RTCRtpTransceiverDirection::Sendrecv,
            //             send_encodings: vec![
            //                 RTCRtpEncodingParameters::default(),
            //             ],
            //         }),
            //     )
            //     .await
            //     .unwrap();

            peer_connection.on_track(Box::new(move |track, a, c| {
                println!(
                    "New track received: id={:?}, kind={:?}",
                    track.id(),
                    track.kind()
                );
                let mut res = media_track_tx_clone.clone();
                let _ = res.start_send(track);
                Box::pin(async {})
            }));
            spawn_local(async move {
                while let Some(track) = rx_media_track.next().await {
                    println!("New remote track received: {:?}", track.kind());
                    let kind = match track.kind() {
                        RTPCodecType::Audio => MediaKind::Audio,
                        RTPCodecType::Video => MediaKind::Video,
                        _ => continue,
                    };
                    media_tracks
                        .clone()
                        .borrow_mut()
                        .create_track(track.id().to_string(), kind, track)
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
        match self.data_channels.borrow().get_data_channel("DATEX") {
            Some(channel) => Box::pin(async move {
                let bytes = Bytes::from(block.to_vec());
                channel.borrow().data_channel.send(&bytes).await.is_ok()
            }),
            _ => {
                error!("Failed to send message, data channel not found");
                Box::pin(async move { false })
            }
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

impl ComInterfaceFactory<WebRTCInterfaceSetupData> for WebRTCNativeInterface {
    fn create(
        setup_data: WebRTCInterfaceSetupData,
    ) -> Result<WebRTCNativeInterface, ComInterfaceError> {
        if let Some(ice_servers) = setup_data.ice_servers.as_ref() {
            if ice_servers.is_empty() {
                error!(
                    "Ice servers list is empty, at least one ice server is required"
                );
                Err(ComInterfaceError::InvalidSetupData)
            } else {
                Ok(WebRTCNativeInterface::new_with_ice_servers(
                    setup_data.peer_endpoint,
                    ice_servers.to_owned(),
                ))
            }
        } else {
            Ok(WebRTCNativeInterface::new(setup_data.peer_endpoint))
        }
    }

    fn get_default_properties() -> InterfaceProperties {
        InterfaceProperties::default()
    }
}

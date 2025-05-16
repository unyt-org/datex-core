use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
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
        socket_provider::SingleSocketProvider,
    },
    set_opener,
};
use async_trait::async_trait;

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
use log::error;
use webrtc::{
    data_channel::RTCDataChannel,
    peer_connection::RTCPeerConnection,
};
pub struct WebRTCNativeInterface {
    info: ComInterfaceInfo,
    commons: Rc<RefCell<WebRTCCommon>>,
    peer_connection: Rc<Option<RTCPeerConnection>>,
    data_channels: Rc<RefCell<DataChannels<RTCDataChannel>>>,
}
impl SingleSocketProvider for WebRTCNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets()
    }
}
impl WebRTCTrait<RTCDataChannel> for WebRTCNativeInterface {
    fn new(peer_endpoint: impl Into<Endpoint>) -> Self {
        WebRTCNativeInterface {
            info: ComInterfaceInfo::default(),
            commons: Rc::new(RefCell::new(WebRTCCommon::new(peer_endpoint))),
            peer_connection: Rc::new(None),
            data_channels: Rc::new(RefCell::new(DataChannels::new())),
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
impl WebRTCTraitInternal<RTCDataChannel> for WebRTCNativeInterface {
    fn provide_data_channels(
        &self,
    ) -> Rc<RefCell<DataChannels<RTCDataChannel>>> {
        self.data_channels.clone()
    }
    fn provide_info(&self) -> &ComInterfaceInfo {
        &self.info
    }

    async fn handle_create_data_channel(
        &self,
    ) -> Result<DataChannel<RTCDataChannel>, WebRTCError> {
        todo!()
        // if let Some(peer_connection) = self.peer_connection.as_ref() {
        //     let data_channel = peer_connection.create_data_channel("DATEX");
        //     Ok(DataChannel::new(data_channel.label(), data_channel))
        // } else {
        //     error!("Peer connection is not initialized");
        //     Err(WebRTCError::ConnectionError)
        // }
    }

    async fn handle_setup_data_channel(
        channel: Rc<RefCell<DataChannel<RTCDataChannel>>>,
    ) -> Result<(), WebRTCError> {
        todo!()
        // let channel_clone = channel.clone();
        // {
        //     let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        //         if let Some(ref open_channel) =
        //             channel_clone.borrow().open_channel
        //         {
        //             info!("Data channel opened to");
        //             open_channel(channel_clone.clone());
        //         }
        //     });
        //     channel
        //         .clone()
        //         .borrow()
        //         .data_channel
        //         .set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        //     onopen_callback.forget();
        // }
        // let channel_clone = channel.clone();
        // {
        //     let onmessage_callback = Closure::<dyn FnMut(MessageEvent)>::new(
        //         move |message_event: MessageEvent| {
        //             let data_channel = channel_clone.borrow();
        //             if let Some(ref on_message) = data_channel.on_message {
        //                 let data = message_event.data().try_as_u8_slice();
        //                 if let Ok(data) = data {
        //                     on_message(data);
        //                 }
        //             }
        //         },
        //     );
        //     channel.clone().borrow().data_channel.set_onmessage(Some(
        //         onmessage_callback.as_ref().unchecked_ref(),
        //     ));
        //     onmessage_callback.forget();
        // }
        // Ok(())
    }

    async fn handle_create_offer(
        &self,
    ) -> Result<RTCSessionDescriptionDX, WebRTCError> {
        todo!()
        // if let Some(peer_connection) = self.peer_connection.as_ref() {
        //     let offer = JsFuture::from(peer_connection.create_offer())
        //         .await
        //         .unwrap();
        //     let sdp: String = Reflect::get(&offer, &JsValue::from_str("sdp"))
        //         .unwrap()
        //         .as_string()
        //         .unwrap();
        //     info!("Offer created {sdp}");
        //     Ok(RTCSessionDescriptionDX {
        //         sdp_type: RTCSdpTypeDX::Offer,
        //         sdp,
        //     })
        // } else {
        //     error!("Peer connection is not initialized");
        //     Err(WebRTCError::ConnectionError)
        // }
    }

    async fn handle_add_ice_candidate(
        &self,
        candidate_init: RTCIceCandidateInitDX,
    ) -> Result<(), WebRTCError> {
        todo!()
        // if let Some(peer_connection) = self.peer_connection.as_ref() {
        //     let signaling_state = peer_connection.signaling_state();

        //     // Ensure remote description is set
        //     if signaling_state != RtcSignalingState::Stable
        //         && signaling_state != RtcSignalingState::HaveLocalOffer
        //         && signaling_state != RtcSignalingState::HaveRemoteOffer
        //     {
        //         return Err(WebRTCError::MissingRemoteDescription);
        //     }
        //     let js_candidate_init =
        //         RtcIceCandidateInit::new(&candidate_init.candidate);
        //     js_candidate_init.set_sdp_mid(candidate_init.sdp_mid.as_deref());
        //     js_candidate_init
        //         .set_sdp_m_line_index(candidate_init.sdp_mline_index);
        //     info!(
        //         "Adding ICE candidate for {}: {:?}",
        //         self.remote_endpoint(),
        //         js_candidate_init
        //     );
        //     JsFuture::from(
        //         peer_connection
        //             .add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(
        //                 &js_candidate_init,
        //             )),
        //     )
        //     .await
        //     .map_err(|e| {
        //         error!("Failed to add ICE candidate {e:?}");
        //         WebRTCError::InvalidCandidate
        //     })?;
        //     info!("ICE candidate added {}", self.remote_endpoint());
        //     Ok(())
        // } else {
        //     error!("Peer connection is not initialized");
        //     Err(WebRTCError::ConnectionError)
        // }
    }

    async fn handle_set_local_description(
        &self,
        description: RTCSessionDescriptionDX,
    ) -> Result<(), WebRTCError> {
        todo!()
        // if let Some(peer_connection) = self.peer_connection.as_ref() {
        //     let description_init =
        //         RtcSessionDescriptionInit::new(match description.sdp_type {
        //             RTCSdpTypeDX::Offer => RtcSdpType::Offer,
        //             RTCSdpTypeDX::Answer => RtcSdpType::Answer,
        //             _ => Err(WebRTCError::InvalidSdp)?,
        //         });
        //     description_init.set_sdp(&description.sdp);
        //     JsFuture::from(
        //         peer_connection.set_local_description(&description_init),
        //     )
        //     .await
        //     .unwrap();
        //     Ok(())
        // } else {
        //     error!("Peer connection is not initialized");
        //     return Err(WebRTCError::ConnectionError);
        // }
    }

    async fn handle_set_remote_description(
        &self,
        description: RTCSessionDescriptionDX,
    ) -> Result<(), WebRTCError> {
        todo!()
        // if let Some(peer_connection) = self.peer_connection.as_ref() {
        //     let description_init =
        //         RtcSessionDescriptionInit::new(match description.sdp_type {
        //             RTCSdpTypeDX::Offer => RtcSdpType::Offer,
        //             RTCSdpTypeDX::Answer => RtcSdpType::Answer,
        //             _ => Err(WebRTCError::InvalidSdp)?,
        //         });
        //     description_init.set_sdp(&description.sdp);
        //     JsFuture::from(
        //         peer_connection.set_remote_description(&description_init),
        //     )
        //     .await
        //     .unwrap();
        //     Ok(())
        // } else {
        //     error!("Peer connection is not initialized");
        //     return Err(WebRTCError::ConnectionError);
        // }
    }

    async fn handle_create_answer(
        &self,
    ) -> Result<RTCSessionDescriptionDX, WebRTCError> {
        todo!()
        // if let Some(peer_connection) = self.peer_connection.as_ref() {
        //     let answer = JsFuture::from(peer_connection.create_answer())
        //         .await
        //         .unwrap();
        //     let sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))
        //         .unwrap()
        //         .as_string()
        //         .unwrap();
        //     Ok(RTCSessionDescriptionDX {
        //         sdp_type: RTCSdpTypeDX::Answer,
        //         sdp,
        //     })
        // } else {
        //     error!("Peer connection is not initialized");
        //     Err(WebRTCError::ConnectionError)
        // }
    }

    fn get_commons(&self) -> Rc<RefCell<WebRTCCommon>> {
        self.commons.clone()
    }
}

#[com_interface]
impl WebRTCNativeInterface {
    #[create_opener]
    async fn open(&mut self) -> Result<(), WebRTCError> {
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
        let success = {
            if let Some(channel) =
                self.data_channels.borrow().get_data_channel("DATEX")
            {
                true
            } else {
                error!("Failed to send message, data channel not found");
                false
            }
        };
        Box::pin(async move { success })
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

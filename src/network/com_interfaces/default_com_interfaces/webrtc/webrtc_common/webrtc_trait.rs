use crate::std_sync::Mutex;
use crate::stdlib::{cell::RefCell, rc::Rc, sync::Arc};

use async_trait::async_trait;
use futures::channel::oneshot;
use log::{error, info};

use crate::{
    network::com_interfaces::{
        com_interface::{
            ComInterfaceInfo, ComInterfaceUUID,
        },
        default_com_interfaces::webrtc::webrtc_common::media_tracks::{
            MediaKind, MediaTrack, MediaTracks,
        },
    },
    serde::{deserializer::from_bytes, serializer::to_bytes},
    values::core_values::endpoint::Endpoint,
};
use crate::network::com_interfaces::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::properties::InterfaceDirection;
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::com_interface::socket_manager::ComInterfaceSocketManager;
use crate::task::UnboundedSender;
use super::{
    data_channels::{DataChannel, DataChannels},
    structures::{
        RTCIceCandidateInitDX, RTCIceServer, RTCSessionDescriptionDX,
    },
    utils::WebRTCError,
    webrtc_commons::WebRTCCommon,
};

#[async_trait(?Send)]
pub trait WebRTCTraitInternal<DC: 'static, MR: 'static, ML: 'static> {
    // These method must be implemented in the interface
    fn provide_data_channels(&self) -> Rc<RefCell<DataChannels<DC>>>;
    fn provide_remote_media_tracks(&self) -> Rc<RefCell<MediaTracks<MR>>>;
    fn provide_local_media_tracks(&self) -> Rc<RefCell<MediaTracks<ML>>>;
    fn get_commons(&self) -> Arc<Mutex<WebRTCCommon>>;
    fn provide_com_interface(&self) -> &Rc<RefCell<ComInterface>>;
    async fn handle_create_data_channel(
        &self,
    ) -> Result<DataChannel<DC>, WebRTCError>;
    async fn handle_create_media_channel(
        &self,
        id: String,
        kind: MediaKind,
    ) -> Result<MediaTrack<ML>, WebRTCError>;

    async fn handle_setup_data_channel(
        channel: Rc<RefCell<DataChannel<DC>>>,
    ) -> Result<(), WebRTCError>;

    async fn handle_setup_media_channel(
        channel: Rc<RefCell<MediaTrack<MR>>>,
    ) -> Result<(), WebRTCError>;

    async fn handle_create_offer(
        &self,
    ) -> Result<RTCSessionDescriptionDX, WebRTCError>;
    async fn handle_add_ice_candidate(
        &self,
        candidate: RTCIceCandidateInitDX,
    ) -> Result<(), WebRTCError>;
    async fn handle_set_local_description(
        &self,
        description: RTCSessionDescriptionDX,
    ) -> Result<(), WebRTCError>;
    async fn handle_set_remote_description(
        &self,
        description: RTCSessionDescriptionDX,
    ) -> Result<(), WebRTCError>;
    async fn handle_create_answer(
        &self,
    ) -> Result<RTCSessionDescriptionDX, WebRTCError>;

    fn set_on_ice_candidate(&self, on_ice_candidate: Box<dyn Fn(Vec<u8>)>) {
        self.get_commons().try_lock().unwrap().on_ice_candidate =
            Some(on_ice_candidate);
    }
    fn on_ice_candidate(&self, candidate: RTCIceCandidateInitDX) {
        let commons = self.get_commons();
        commons.try_lock().unwrap().on_ice_candidate(candidate);
    }
    async fn add_ice_candidate(
        &self,
        candidate: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        let is_remote_description_set = {
            let commons = self.get_commons();
            let commons = commons.try_lock().unwrap();
            commons.is_remote_description_set
        };
        if is_remote_description_set {
            let candidate = from_bytes::<RTCIceCandidateInitDX>(&candidate)
                .map_err(|_| WebRTCError::InvalidCandidate)?;
            self.handle_add_ice_candidate(candidate).await?;
        } else {
            let info = self.get_commons();
            info.try_lock().unwrap().candidates.push_back(candidate);
        }
        Ok(())
    }

    fn add_socket(
        manager: Arc<Mutex<ComInterfaceSocketManager>>,
        endpoint: Endpoint,
    ) -> (ComInterfaceSocketUUID, UnboundedSender<Vec<u8>>) {
        let mut manager = manager.try_lock().unwrap();
        let (socket_uuid, sender) = manager
            .create_and_init_socket(InterfaceDirection::InOut, 1);
        manager
            .register_socket_with_endpoint(socket_uuid.clone(), endpoint, 1)
            .unwrap();

        (socket_uuid, sender)
    }
    fn _remote_endpoint(&self) -> Endpoint {
        self.get_commons().try_lock().unwrap().endpoint.clone()
    }
    async fn set_remote_description(
        &self,
        description: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        let description = from_bytes::<RTCSessionDescriptionDX>(&description)
            .map_err(|_| WebRTCError::InvalidSdp)?;
        self.handle_set_remote_description(description).await?;
        self.get_commons()
            .try_lock()
            .unwrap()
            .is_remote_description_set = true;
        let candidates = {
            let commons = self.get_commons();
            let mut commons = commons.try_lock().unwrap();

            commons.candidates.drain(..).collect::<Vec<_>>()
        };
        for candidate in candidates {
            if let Ok(candidate) =
                from_bytes::<RTCIceCandidateInitDX>(&candidate)
            {
                self.handle_add_ice_candidate(candidate).await?;
            } else {
                error!("Failed to deserialize candidate");
            }
        }
        Ok(())
    }

    async fn setup_data_channel(
        commons: Arc<Mutex<WebRTCCommon>>,
        endpoint: Endpoint,
        data_channels: Rc<RefCell<DataChannels<DC>>>,
        channel: Rc<RefCell<DataChannel<DC>>>,
        com_interface_manager: Arc<Mutex<ComInterfaceSocketManager>>,
    ) -> Result<(), WebRTCError> {
        let channel_clone = channel.clone();
        let channel_clone2 = channel.clone();

        let (socket_uuid, mut sender) = Self::add_socket(
            com_interface_manager,
            endpoint.clone(),
        );

        channel
            .borrow_mut()
            .open_channel
            .borrow_mut()
            .replace(Box::new(move || {
                info!("Data channel opened and added to data channels");


                // FIXME #204
                let data_channels = data_channels.clone();
                let channel_clone2 = channel_clone2.clone();
                channel_clone2
                    .clone()
                    .borrow()
                    .set_socket_uuid(socket_uuid.clone());

                data_channels
                    .borrow_mut()
                    .add_data_channel(channel_clone2.clone());

                if let Some(on_connect) =
                    commons.try_lock().unwrap().on_connect.as_ref()
                {
                    on_connect();
                }

            }));
        channel
            .borrow_mut()
            .on_message
            .borrow_mut()
            .replace(Box::new(move |data| {
                let data = data.to_vec();
                if let Some(socket_uuid) =
                    channel_clone.borrow().get_socket_uuid()
                {
                    info!(
                            "Received data on socket: {data:?} {socket_uuid}"
                        );
                    sender
                        .start_send(data)
                        .unwrap();
                }
            }));
        Self::handle_setup_data_channel(channel).await?;
        Ok(())
    }
}

#[async_trait(?Send)]
pub trait WebRTCTrait<DC: 'static, MR: 'static, ML: 'static>:
    WebRTCTraitInternal<DC, MR, ML>
{
    fn new(
        peer_endpoint: impl Into<Endpoint>,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Self;
    fn new_with_ice_servers(
        peer_endpoint: impl Into<Endpoint>,
        ice_servers: Vec<RTCIceServer>,
        com_interface: Rc<RefCell<ComInterface>>,
    ) -> Self;
    async fn create_offer(&self) -> Result<Vec<u8>, WebRTCError> {
        let data_channel = self.handle_create_data_channel().await?;
        let data_channel_rc = Rc::new(RefCell::new(data_channel));
        let data_channels = self.provide_data_channels();
        {
            Self::setup_data_channel(
                self.get_commons(),
                self._remote_endpoint(),
                data_channels,
                data_channel_rc.clone(),
                self.provide_com_interface().borrow().socket_manager(),
            )
            .await?;
        }
        let offer = self.handle_create_offer().await?;
        self.handle_set_local_description(offer.clone()).await?;
        let offer = to_bytes(&offer).unwrap();
        Ok(offer)
    }
    async fn create_answer(
        &self,
        offer: Vec<u8>,
    ) -> Result<Vec<u8>, WebRTCError> {
        self.set_remote_description(offer).await?;
        let answer = self.handle_create_answer().await?;
        self.handle_set_local_description(answer.clone()).await?;
        let answer = to_bytes(&answer).unwrap();
        Ok(answer)
    }
    async fn wait_for_connection(&self) -> Result<(), WebRTCError> {
        {
            let is_connected = self
                .provide_data_channels()
                .borrow()
                .data_channels
                .values()
                .len()
                > 0;
            if is_connected {
                return Ok(());
            }
        }
        let (tx, rx) = oneshot::channel();
        let tx_clone = RefCell::new(Some(tx));
        {
            let commons = self.get_commons();
            let mut commons = commons.try_lock().unwrap();
            commons.on_connect = Some(Box::new(move || {
                info!("Connected");
                tx_clone.take().unwrap().send(()).unwrap();
            }));
        }
        rx.await.map_err(|_| {
            error!("Failed to receive connection signal");
            WebRTCError::ConnectionError
        })?;
        Ok(())
    }

    async fn set_answer(&self, answer: Vec<u8>) -> Result<(), WebRTCError> {
        self.set_remote_description(answer).await
    }
    // This must be called in the open method
    fn setup_listeners(&self) {
        let data_channels = self.provide_data_channels();
        let data_channels_clone = data_channels.clone();

        let commons = self.get_commons();
        let remote_endpoint = self.remote_endpoint();
        let com_interface_socket_manager = self.provide_com_interface().borrow().socket_manager();

        data_channels.borrow_mut().on_add =
            Some(Box::new(move |data_channel| {
                let data_channel = data_channel.clone();
                let data_channels_clone = data_channels_clone.clone();
                let remote_endpoint = remote_endpoint.clone();
                let commons = commons.clone();
                let com_interface_socket_manager =
                    com_interface_socket_manager.clone();
                Box::pin(async move {
                    Self::setup_data_channel(
                        commons,
                        remote_endpoint.clone(),
                        data_channels_clone.clone(),
                        data_channel,
                        com_interface_socket_manager.clone(),
                    )
                    .await
                    .unwrap()
                })
            }));

        let media_tracks = self.provide_remote_media_tracks();
        media_tracks.borrow_mut().on_add = Some(Box::new(move |media_track| {
            let media_track = media_track.clone();
            Box::pin(async move {
                Self::handle_setup_media_channel(media_track.clone())
                    .await
                    .unwrap();
            })
        }));
    }

    fn set_ice_servers(&self, ice_servers: Vec<RTCIceServer>) {
        let commons = self.get_commons();
        let mut commons = commons.try_lock().unwrap();
        commons.ice_servers = ice_servers;
    }
    fn remote_endpoint(&self) -> Endpoint {
        self._remote_endpoint()
    }

    async fn create_media_track(
        &self,
        id: String,
        kind: MediaKind,
    ) -> Result<Rc<RefCell<MediaTrack<ML>>>, WebRTCError> {
        let channel = self.handle_create_media_channel(id, kind).await?;
        let channel_rc = Rc::new(RefCell::new(channel));
        let media_channels = self.provide_local_media_tracks();
        media_channels
            .borrow_mut()
            .tracks
            .insert(channel_rc.borrow().id(), channel_rc.clone());
        Ok(channel_rc)
    }
}

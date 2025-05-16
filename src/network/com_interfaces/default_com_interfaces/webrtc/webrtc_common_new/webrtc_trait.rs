use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use axum::async_trait;
use log::{error, info};

use crate::{
    datex_values::Endpoint,
    network::com_interfaces::{
        com_interface::{
            ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
        },
        com_interface_properties::InterfaceDirection,
        com_interface_socket::{ComInterfaceSocket, ComInterfaceSocketUUID},
    },
};

use super::{
    data_channels::{DataChannel, DataChannels},
    structures::{
        RTCIceCandidateInitDX, RTCIceServer, RTCSessionDescriptionDX,
    },
    utils::{deserialize, serialize, WebRTCError},
    webrtc_commons::WebRTCCommon,
};

#[async_trait(?Send)]
pub trait PubWebRTCTrait<T: 'static> {
    fn new(peer_endpoint: impl Into<Endpoint>) -> Self;
    fn new_with_ice_servers(
        peer_endpoint: impl Into<Endpoint>,
        ice_servers: Vec<RTCIceServer>,
    ) -> Self;
}

#[async_trait(?Send)]
pub trait WebRTCTrait<T: 'static> {
    // These method must be implemented in the interface
    fn provide_data_channels(&self) -> Rc<RefCell<DataChannels<T>>>;
    fn get_commons(&self) -> Rc<RefCell<WebRTCCommon>>;
    fn provide_info(&self) -> &ComInterfaceInfo;

    async fn handle_create_data_channel(
        &self,
    ) -> Result<DataChannel<T>, WebRTCError>;
    async fn handle_setup_data_channel(
        channel: Rc<RefCell<DataChannel<T>>>,
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

    // This must be called in the open method
    fn setup_listeners(&self) {
        let data_channels = self.provide_data_channels();
        let data_channels_clone = data_channels.clone();

        let info = self.provide_info();
        let interface_uuid = info.get_uuid().clone();
        let sockets = info.com_interface_sockets();

        let remote_endpoint = self.remote_endpoint();
        data_channels.borrow_mut().on_add =
            Some(Box::new(move |data_channel| {
                let data_channel = data_channel.clone();
                let data_channels_clone = data_channels_clone.clone();
                let sockets = sockets.clone();
                let interface_uuid = interface_uuid.clone();
                let remote_endpoint = remote_endpoint.clone();
                Box::pin(async move {
                    Self::setup_data_channel(
                        remote_endpoint.clone(),
                        interface_uuid.clone(),
                        sockets.clone(),
                        data_channels_clone.clone(),
                        data_channel,
                    )
                    .await
                    .unwrap()
                })
            }));
    }
    fn set_ice_servers(&self, ice_servers: Vec<RTCIceServer>) {
        let commons = self.get_commons();
        let mut commons = commons.borrow_mut();
        commons.ice_servers = ice_servers;
    }
    fn remote_endpoint(&self) -> Endpoint {
        self.get_commons().borrow().endpoint.clone()
    }
    fn set_on_ice_candidate(&self, on_ice_candidate: Box<dyn Fn(Vec<u8>)>) {
        self.get_commons().borrow_mut().on_ice_candidate =
            Some(on_ice_candidate);
    }
    fn on_ice_candidate(&self, candidate: RTCIceCandidateInitDX) {
        let commons = self.get_commons();
        commons.borrow().on_ice_candidate(candidate);
    }
    async fn add_ice_candidate(
        &self,
        candidate: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        if self.get_commons().borrow().is_remote_description_set {
            let candidate = deserialize::<RTCIceCandidateInitDX>(&candidate)
                .map_err(|_| WebRTCError::InvalidCandidate)?;
            self.handle_add_ice_candidate(candidate).await?;
        } else {
            let info = self.get_commons();
            info.borrow_mut().candidates.push_back(candidate);
        }
        Ok(())
    }

    fn add_socket(
        endpoint: Endpoint,
        interface_uuid: ComInterfaceUUID,
        sockets: Arc<Mutex<ComInterfaceSockets>>,
    ) -> ComInterfaceSocketUUID {
        // FIXME clean up old sockets
        let mut sockets = sockets.lock().unwrap();
        let socket = ComInterfaceSocket::new(
            interface_uuid,
            InterfaceDirection::InOut,
            1,
        );
        let socket_uuid = socket.uuid.clone();
        sockets.add_socket(Arc::new(Mutex::new(socket)));
        sockets
            .register_socket_endpoint(socket_uuid.clone(), endpoint, 1)
            .expect("Failed to register socket endpoint");
        socket_uuid
    }
    async fn create_offer(&self) -> Result<Vec<u8>, WebRTCError> {
        let data_channel = self.handle_create_data_channel().await?;
        let data_channel_rc = Rc::new(RefCell::new(data_channel));
        let data_channels = self.provide_data_channels();
        {
            let info = self.provide_info();
            let interface_uuid = info.get_uuid().clone();
            let sockets = info.com_interface_sockets();
            Self::setup_data_channel(
                self.remote_endpoint(),
                interface_uuid,
                sockets.clone(),
                data_channels,
                data_channel_rc.clone(),
            )
            .await?;
        }
        let offer = self.handle_create_offer().await?;
        self.handle_set_local_description(offer.clone()).await?;
        let offer = serialize(&offer).unwrap();
        Ok(offer)
    }
    async fn create_answer(
        &self,
        offer: Vec<u8>,
    ) -> Result<Vec<u8>, WebRTCError> {
        self.set_remote_description(offer).await?;
        let answer = self.handle_create_answer().await?;
        self.handle_set_local_description(answer.clone()).await?;
        let answer = serialize(&answer).unwrap();
        Ok(answer)
    }
    async fn set_remote_description(
        &self,
        description: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        let description = deserialize::<RTCSessionDescriptionDX>(&description)
            .map_err(|_| WebRTCError::InvalidSdp)?;
        self.handle_set_remote_description(description).await?;
        self.get_commons().borrow_mut().is_remote_description_set = true;
        let candidates = {
            let commons = self.get_commons();
            let mut commons = commons.borrow_mut();
            let candidates = commons.candidates.drain(..).collect::<Vec<_>>();
            candidates
        };
        for candidate in candidates {
            if let Ok(candidate) =
                deserialize::<RTCIceCandidateInitDX>(&candidate)
            {
                self.handle_add_ice_candidate(candidate).await?;
            } else {
                error!("Failed to deserialize candidate");
            }
        }
        Ok(())
    }
    async fn setup_data_channel(
        endpoint: Endpoint,
        interface_uuid: ComInterfaceUUID,
        sockets: Arc<Mutex<ComInterfaceSockets>>,
        data_channels: Rc<RefCell<DataChannels<T>>>,
        channel: Rc<RefCell<DataChannel<T>>>,
    ) -> Result<(), WebRTCError> {
        let channel_clone = channel.clone();
        let sockets_clone = sockets.clone();
        channel.borrow_mut().open_channel =
            Some(Box::new(move |channel: Rc<RefCell<DataChannel<T>>>| {
                info!("Data channel opened and added to data channels");
                let socket_uuid = Self::add_socket(
                    endpoint.clone(),
                    interface_uuid.clone(),
                    sockets.clone(),
                );
                channel
                    .clone()
                    .borrow()
                    .set_socket_uuid(socket_uuid.clone());
                data_channels.borrow_mut().add_data_channel(channel);
            }));

        channel.borrow_mut().on_message = Some(Box::new(move |data| {
            let data = data.to_vec();
            if let Some(socket_uuid) = channel_clone.borrow().get_socket_uuid()
            {
                let sockets = sockets_clone.lock().unwrap();
                if let Some(socket) = sockets.sockets.get(&socket_uuid) {
                    info!("Received data on socket: {data:?} {socket_uuid}");
                    socket
                        .lock()
                        .unwrap()
                        .receive_queue
                        .lock()
                        .unwrap()
                        .extend(data);
                }
            }
        }));
        Self::handle_setup_data_channel(channel).await?;
        Ok(())
    }
    async fn set_answer(&self, answer: Vec<u8>) -> Result<(), WebRTCError> {
        self.set_remote_description(answer).await
    }
}

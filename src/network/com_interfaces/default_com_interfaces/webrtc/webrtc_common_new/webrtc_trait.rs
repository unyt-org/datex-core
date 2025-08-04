use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures::channel::oneshot;
use log::{error, info};

use crate::{
    values::core_values::endpoint::Endpoint,
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
pub trait WebRTCTraitInternal<T: 'static> {
    // These method must be implemented in the interface
    fn provide_data_channels(&self) -> Rc<RefCell<DataChannels<T>>>;
    fn get_commons(&self) -> Arc<Mutex<WebRTCCommon>>;
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

    fn set_on_ice_candidate(&self, on_ice_candidate: Box<dyn Fn(Vec<u8>)>) {
        self.get_commons().lock().unwrap().on_ice_candidate =
            Some(on_ice_candidate);
    }
    fn on_ice_candidate(&self, candidate: RTCIceCandidateInitDX) {
        let commons = self.get_commons();
        commons.lock().unwrap().on_ice_candidate(candidate);
    }
    async fn add_ice_candidate(
        &self,
        candidate: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        let is_remote_description_set = {
            let commons = self.get_commons();
            let commons = commons.lock().unwrap();
            commons.is_remote_description_set
        };
        if is_remote_description_set {
            let candidate = deserialize::<RTCIceCandidateInitDX>(&candidate)
                .map_err(|_| WebRTCError::InvalidCandidate)?;
            self.handle_add_ice_candidate(candidate).await?;
        } else {
            let info = self.get_commons();
            info.lock().unwrap().candidates.push_back(candidate);
        }
        Ok(())
    }

    fn add_socket(
        endpoint: Endpoint,
        interface_uuid: ComInterfaceUUID,
        sockets: Arc<Mutex<ComInterfaceSockets>>,
    ) -> ComInterfaceSocketUUID {
        // FIXME #203 clean up old sockets
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
    fn _remote_endpoint(&self) -> Endpoint {
        self.get_commons().lock().unwrap().endpoint.clone()
    }
    async fn set_remote_description(
        &self,
        description: Vec<u8>,
    ) -> Result<(), WebRTCError> {
        let description = deserialize::<RTCSessionDescriptionDX>(&description)
            .map_err(|_| WebRTCError::InvalidSdp)?;
        self.handle_set_remote_description(description).await?;
        self.get_commons().lock().unwrap().is_remote_description_set = true;
        let candidates = {
            let commons = self.get_commons();
            let mut commons = commons.lock().unwrap();
            
            commons.candidates.drain(..).collect::<Vec<_>>()
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
        commons: Arc<Mutex<WebRTCCommon>>,
        endpoint: Endpoint,
        interface_uuid: ComInterfaceUUID,
        sockets: Arc<Mutex<ComInterfaceSockets>>,
        data_channels: Rc<RefCell<DataChannels<T>>>,
        channel: Rc<RefCell<DataChannel<T>>>,
    ) -> Result<(), WebRTCError> {
        let channel_clone = channel.clone();
        let channel_clone2 = channel.clone();
        let sockets_clone = sockets.clone();

        channel
            .borrow_mut()
            .open_channel
            .borrow_mut()
            .replace(Box::new(move || {
                info!("Data channel opened and added to data channels");

                let socket_uuid = Self::add_socket(
                    endpoint.clone(),
                    interface_uuid.clone(),
                    sockets.clone(),
                );
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
                    commons.lock().unwrap().on_connect.as_ref()
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
                    let sockets = sockets_clone.lock().unwrap();
                    if let Some(socket) = sockets.sockets.get(&socket_uuid) {
                        info!(
                            "Received data on socket: {data:?} {socket_uuid}"
                        );
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
}

#[async_trait(?Send)]
pub trait WebRTCTrait<T: 'static>: WebRTCTraitInternal<T> {
    fn new(peer_endpoint: impl Into<Endpoint>) -> Self;
    fn new_with_ice_servers(
        peer_endpoint: impl Into<Endpoint>,
        ice_servers: Vec<RTCIceServer>,
    ) -> Self;
    async fn create_offer(&self) -> Result<Vec<u8>, WebRTCError> {
        let data_channel = self.handle_create_data_channel().await?;
        let data_channel_rc = Rc::new(RefCell::new(data_channel));
        let data_channels = self.provide_data_channels();
        {
            let info = self.provide_info();
            let interface_uuid = info.get_uuid().clone();
            let sockets = info.com_interface_sockets();
            Self::setup_data_channel(
                self.get_commons(),
                self._remote_endpoint(),
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
            let mut commons = commons.lock().unwrap();
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

        let info = self.provide_info();
        let interface_uuid = info.get_uuid().clone();
        let sockets = info.com_interface_sockets();
        let commons = self.get_commons();
        let remote_endpoint = self.remote_endpoint();
        data_channels.borrow_mut().on_add =
            Some(Box::new(move |data_channel| {
                let data_channel = data_channel.clone();
                let data_channels_clone = data_channels_clone.clone();
                let sockets = sockets.clone();
                let interface_uuid = interface_uuid.clone();
                let remote_endpoint = remote_endpoint.clone();
                let commons = commons.clone();
                Box::pin(async move {
                    Self::setup_data_channel(
                        commons,
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
        let mut commons = commons.lock().unwrap();
        commons.ice_servers = ice_servers;
    }
    fn remote_endpoint(&self) -> Endpoint {
        self._remote_endpoint()
    }
}

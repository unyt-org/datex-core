use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use strum::Display;
use thiserror::Error;

use crate::values::core_values::endpoint::Endpoint;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct RTCIceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[async_trait(?Send)]
pub trait WebRTCInterfaceTrait {
    fn new(endpoint: impl Into<Endpoint>) -> Self;
    fn set_ice_servers(self, ice_servers: Vec<RTCIceServer>) -> Self;
    fn new_with_media_support(endpoint: impl Into<Endpoint>) -> Self;
    async fn create_offer(&self, use_reliable_connection: bool) -> Vec<u8>;
    async fn set_remote_description(
        &self,
        description: Vec<u8>,
    ) -> Result<(), WebRTCError>;
    async fn add_ice_candidate(
        &self,
        candidate: Vec<u8>,
    ) -> Result<(), WebRTCError>;
    async fn create_answer(&self) -> Vec<u8>;
}

#[derive(Debug, Display, Error)]
pub enum WebRTCError {
    Unsupported,
    InvalidURL,
    ConnectionError,
    SendError,
    ReceiveError,
    MissingRemoteDescription,

    InvalidCandidate,
    InvalidSdp,
    MediaEngineError,
}

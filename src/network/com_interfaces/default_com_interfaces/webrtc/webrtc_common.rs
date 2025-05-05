use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use strum::Display;
use thiserror::Error;

use crate::datex_values::Endpoint;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct RTCIceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
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

// FIXME this will later be replaced with a proper implementation
// of Datex Values
pub fn serialize<T: Serialize>(
    value: &T,
) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_string(value).map(|s| s.into_bytes())
}

// FIXME this will later be replaced with a proper implementation
// of Datex Values
pub fn deserialize<T: DeserializeOwned>(
    value: &[u8],
) -> Result<T, serde_json::Error> {
    let string = std::str::from_utf8(value).unwrap();
    serde_json::from_str(string)
}

#[derive(Debug, Display, Error)]
pub enum WebRTCError {
    Unsupported,
    InvalidURL,
    ConnectionError,
    SendError,
    ReceiveError,

    InvalidCandidate,
    InvalidSdp,
    MediaEngineError,
}

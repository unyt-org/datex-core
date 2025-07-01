use serde::{de::DeserializeOwned, Serialize};
use strum::Display;
use thiserror::Error;

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
    MissingRemoteDescription,

    InvalidCandidate,
    InvalidSdp,
    MediaEngineError,
}

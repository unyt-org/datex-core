use strum::Display;
use thiserror::Error;

use crate::network::com_hub::errors::ComHubError;

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
    ComHubError(ComHubError),
}

use strum::Display;
use thiserror::Error;

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

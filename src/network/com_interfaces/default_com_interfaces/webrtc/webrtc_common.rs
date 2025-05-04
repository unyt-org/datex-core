use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum WebRTCError {
    InvalidURL,
    ConnectionError,
    SendError,
    ReceiveError,

    InvalidCandidate,
    InvalidSdp,
    MediaEngineError,
}

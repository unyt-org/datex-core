use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum SerialError {
    Other(String),
    InvalidAddress,
    ConnectionError,
    SendError,
    ReceiveError,
}

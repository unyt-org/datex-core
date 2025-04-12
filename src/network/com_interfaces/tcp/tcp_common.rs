use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum TCPError {
    Other(String),
    InvalidURL,
    ConnectionError,
    SendError,
    ReceiveError,
}

use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum HTTPError {
    Other(String),
    InvalidAddress,
    ConnectionError,
    SendError,
    ReceiveError,
}

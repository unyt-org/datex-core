use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum TCPError {
    InvalidURL,
    ConnectionError,
    SendError,
    ReceiveError,
}

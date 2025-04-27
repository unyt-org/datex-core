use strum::Display;
use thiserror::Error;

pub struct TCPClientInterfaceSetupData {
    pub address: String,
}

pub struct TCPServerInterfaceSetupData {
    pub port: u16,
}

#[derive(Debug, Display, Error, Clone, PartialEq)]
pub enum TCPError {
    Other(String),
    InvalidURL,
    ConnectionError,
    SendError,
    ReceiveError,
}

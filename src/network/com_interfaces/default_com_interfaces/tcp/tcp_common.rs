use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct TCPClientInterfaceSetupData {
    pub address: String,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
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

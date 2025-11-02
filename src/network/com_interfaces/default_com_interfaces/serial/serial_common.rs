use core::prelude::rust_2024::*;
use core::result::Result;
use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct SerialInterfaceSetupData {
    pub port_name: Option<String>,
    pub baud_rate: u32,
}

#[derive(Debug, Display, Error)]
pub enum SerialError {
    Other(String),
    PermissionError,
    PortNotFound,
    ConnectionError,
    SendError,
    ReceiveError,
}

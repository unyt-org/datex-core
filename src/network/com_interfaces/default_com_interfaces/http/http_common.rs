use crate::stdlib::string::String;
use core::prelude::rust_2024::*;
use serde::Serialize;
use strum::Display;
use thiserror::Error;
use crate::serde::Deserialize;

#[derive(Debug, Display, Error)]
pub enum HTTPError {
    Other(String),
    InvalidAddress,
    ConnectionError,
    SendError,
    ReceiveError,
}


#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub struct HTTPServerInterfaceSetupData {
    pub port: u16,
}

use core::prelude::rust_2024::*;
use crate::stdlib::string::String;
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

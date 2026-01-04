use crate::stdlib::string::String;
use core::prelude::rust_2024::*;
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

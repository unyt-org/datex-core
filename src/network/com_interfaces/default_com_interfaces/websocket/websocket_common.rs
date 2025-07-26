use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;
use url::Url;
use crate::network::com_hub::ComHubError;

#[derive(Serialize, Deserialize)]
pub struct WebSocketClientInterfaceSetupData {
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct WebSocketServerInterfaceSetupData {
    pub port: u16,
}

#[derive(Debug)]
pub enum URLError {
    InvalidURL,
    InvalidScheme,
}

#[derive(Debug, Display, Error, Clone, PartialEq)]
pub enum WebSocketError {
    Other(String),
    InvalidURL,
    ConnectionError,
    SendError,
    ReceiveError,
}

#[derive(Debug, Display, Error, Clone, PartialEq)]
pub enum WebSocketServerError {
    WebSocketError(WebSocketError),
    InvalidPort,
    ComHubError(ComHubError),
}

impl From<ComHubError> for WebSocketServerError {
    fn from(err: ComHubError) -> Self {
        WebSocketServerError::ComHubError(err)
    }
}


pub fn parse_url(address: &str) -> Result<Url, URLError> {
    let address = if address.contains("://") {
        address.to_string()
    } else {
        format!("wss://{address}")
    };

    let mut url = Url::parse(&address).map_err(|_| URLError::InvalidURL)?;
    match url.scheme() {
        "https" => url.set_scheme("wss").unwrap(),
        "http" => url.set_scheme("ws").unwrap(),
        "wss" | "ws" => (),
        _ => return Err(URLError::InvalidScheme),
    }
    Ok(url)
}

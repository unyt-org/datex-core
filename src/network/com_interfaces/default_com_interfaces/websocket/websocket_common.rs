use strum::Display;
use thiserror::Error;
use url::Url;

pub struct WebSocketClientInterfaceSetupData {
    pub address: String,
}

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

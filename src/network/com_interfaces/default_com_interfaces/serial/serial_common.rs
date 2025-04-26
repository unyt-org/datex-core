use strum::Display;
use thiserror::Error;

pub struct SerialInterfaceSetupData {
    pub port_name: String,
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

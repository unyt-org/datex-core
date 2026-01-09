use core::fmt::Display;

use crate::network::com_interfaces::com_interface::ComInterfaceError;

#[derive(Debug, Clone, PartialEq)]
pub enum ComHubError {
    InterfaceError(ComInterfaceError),
    InterfaceOpenFailed,
    InterfaceCloseFailed,
    InterfaceAlreadyExists,
    InterfaceDoesNotExist,
    InterfaceNotConnected,
    InterfaceTypeDoesNotExist,
    InvalidInterfaceDirectionForFallbackInterface,
    NoResponse,
    SignatureError,
}

impl From<ComInterfaceError> for ComHubError {
    fn from(err: ComInterfaceError) -> Self {
        ComHubError::InterfaceError(err)
    }
}

impl Display for ComHubError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ComHubError::InterfaceError(_msg) => {
                core::write!(f, "ComHubError: ComInterfaceError")
            }
            ComHubError::InterfaceCloseFailed => {
                core::write!(f, "ComHubError: Failed to close interface")
            }
            ComHubError::InterfaceNotConnected => {
                core::write!(f, "ComHubError: Interface not connected")
            }
            ComHubError::InterfaceDoesNotExist => {
                core::write!(f, "ComHubError: Interface does not exit")
            }
            ComHubError::InterfaceAlreadyExists => {
                core::write!(f, "ComHubError: Interface already exists")
            }
            ComHubError::InterfaceTypeDoesNotExist => {
                core::write!(f, "ComHubError: Type of interface does not exist")
            }
            ComHubError::InvalidInterfaceDirectionForFallbackInterface => {
                core::write!(
                    f,
                    "ComHubError: Invalid direction for fallback interface"
                )
            }
            ComHubError::NoResponse => {
                core::write!(f, "ComHubError: No response")
            }
            ComHubError::InterfaceOpenFailed => {
                core::write!(f, "ComHubError: Failed to open interface")
            }
            ComHubError::SignatureError => {
                core::write!(f, "ComHubError: CryptoError")
            }
        }
    }
}

#[derive(Debug)]
pub enum SocketEndpointRegistrationError {
    SocketDisconnected,
    SocketUninitialized,
    SocketEndpointAlreadyRegistered,
}

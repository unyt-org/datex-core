#[derive(Debug, Clone, PartialEq)]
pub enum ComInterfaceError {
    SocketNotFound,
    SocketAlreadyExists,
    ConnectionError,
    SendError,
    ReceiveError,
    InvalidSetupData,
}

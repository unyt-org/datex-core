use strum::EnumString;

use crate::stdlib::time::Duration;
#[derive(PartialEq, Debug, Clone, EnumString)]
pub enum InterfaceDirection {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone)]
pub struct InterfaceProperties {
    /// the type of the interface, by which it is identified
    /// e.g. "tcp-client", "websocket-server",
    /// multiple interfaces implementations (e.g. for native and web)
    /// can have the same interface type if they are compatible and
    /// have an identical initialization function
    pub interface_type: String,

    /// the channel that the interface is using,
    /// e.g. "tcp", "websocket"
    pub channel: String,

    /// a unique name that further identifies an interface instance
    /// e.g. "wss://example.com:443"
    pub name: Option<String>,

    /// The support message direction of the interface
    pub direction: InterfaceDirection,

    /// Estimated mean latency for this interface type in milliseconds (round trip time).
    /// Lower latency interfaces are preferred over higher latency channels
    pub round_trip_time: Duration,

    /// Bandwidth in bytes per second
    pub max_bandwidth: u32,

    /// If true, the interface does support continuous connections
    pub continuous_connection: bool,

    /// If true, the interface can be used to redirect DATEX messages to other endpoints
    /// which are not directly connected to the interface (default: true)
    /// Currently only enforced for broadcast messages
    pub allow_redirects: bool,

    /// If true, the interface is a secure channel (can not be eavesdropped).
    /// This might be an already encrypted channel such as WebRTC or a channel
    /// that is end-to-end and not interceptable by third parties
    pub is_secure_channel: bool,

    // Defines the reconnection strategy for the interface
    // If the interface is not able to reconnect, it will be destroyed
    pub reconnection_config: ReconnectionConfig,

    /// Timestamp of the interface close event
    /// This is used to determine if the interface shall be reopened
    pub close_timestamp: Option<Duration>,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub enum ReconnectionConfig {
    #[default]
    NoReconnect,
    InstantReconnect,
    ReconnectWithTimeout { timeout: Duration },
    ReconnectWithTimeoutAndAttempts { timeout: Duration, attempts: u8 },
}

impl InterfaceProperties {
    pub fn can_send(&self) -> bool {
        match self.direction {
            InterfaceDirection::In => false,
            InterfaceDirection::Out => true,
            InterfaceDirection::InOut => true,
        }
    }

    pub fn shall_reconnect(&self) -> bool {
        match self.reconnection_config {
            ReconnectionConfig::NoReconnect => false,
            _ => true,
        }
    }

    pub fn can_receive(&self) -> bool {
        match self.direction {
            InterfaceDirection::In => true,
            InterfaceDirection::Out => false,
            InterfaceDirection::InOut => true,
        }
    }
}

impl Default for InterfaceProperties {
    fn default() -> Self {
        InterfaceProperties {
            interface_type: "unknown".to_string(),
            channel: "unknown".to_string(),
            name: None,
            direction: InterfaceDirection::InOut,
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            continuous_connection: false,
            allow_redirects: true,
            is_secure_channel: false,
            reconnection_config: ReconnectionConfig::default(),
            close_timestamp: None,
        }
    }
}

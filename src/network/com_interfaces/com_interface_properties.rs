use core::prelude::rust_2024::*;
use core::time::Duration;
use crate::utils::time::Time;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::{DurationMilliSeconds};
use strum::EnumString;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;

#[derive(PartialEq, Debug, Clone, EnumString, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]

pub enum InterfaceDirection {
    In,
    Out,
    InOut,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
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
    #[serde_as(as = "DurationMilliSeconds")]
    #[cfg_attr(feature = "wasm_runtime", tsify(type = "number"))]
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

    /* private field */
    /// Timestamp of the interface close event
    /// This is used to determine if the interface shall be reopened
    pub close_timestamp: Option<u64>, /*(crate) FIXME */

    /* private field */
    /// Number of reconnection attempts already made
    /// This is used to determine if the interface shall be reopened
    /// and if the interface shall be destroyed
    pub reconnect_attempts: Option<u8>,
}

#[serde_as]
#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
pub enum ReconnectionConfig {
    #[default]
    NoReconnect,
    InstantReconnect,
    ReconnectWithTimeout {
        #[serde_as(as = "DurationMilliSeconds")]
        timeout: Duration,
    },
    ReconnectWithTimeoutAndAttempts {
        #[serde_as(as = "DurationMilliSeconds")]
        timeout: Duration,
        attempts: u8,
    },
}

impl ReconnectionConfig {
    pub fn check_reconnect_timeout(
        close_timestamp: Option<u64>,
        timeout: &Duration,
    ) -> bool {
        let close_timestamp = match close_timestamp {
            Some(ts) => ts,
            None => return false,
        };
        let now = Time::now();
        let elapsed = Duration::from_millis(now - close_timestamp);
        if elapsed < *timeout {
            return false;
        }
        true
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        match self {
            ReconnectionConfig::NoReconnect => None,
            ReconnectionConfig::InstantReconnect => None,
            ReconnectionConfig::ReconnectWithTimeout { timeout } => {
                Some(*timeout)
            }
            ReconnectionConfig::ReconnectWithTimeoutAndAttempts {
                timeout,
                ..
            } => Some(*timeout),
        }
    }

    pub fn get_attempts(&self) -> Option<u8> {
        match self {
            ReconnectionConfig::NoReconnect => None,
            ReconnectionConfig::InstantReconnect => None,
            ReconnectionConfig::ReconnectWithTimeout { .. } => None,
            ReconnectionConfig::ReconnectWithTimeoutAndAttempts {
                attempts,
                ..
            } => Some(*attempts),
        }
    }
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
        !core::matches!(self.reconnection_config, ReconnectionConfig::NoReconnect)
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
            reconnect_attempts: None,
        }
    }
}

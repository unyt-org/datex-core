use std::time::Duration;

#[derive(PartialEq, Debug)]
pub enum InterfaceDirection {
    IN,
    OUT,
    IN_OUT,
}

#[derive(Debug)]
pub struct InterfaceProperties {
    pub channel: String,
    pub name: Option<String>,
    /**
     * Supported communication directions
     */
    pub direction: InterfaceDirection,

    /*
     * Time in milliseconds to wait before reconnecting after a connection error
     */
    pub reconnect_interval: Option<Duration>,

    /**
     * Estimated mean latency for this interface type in milliseconds (round trip time).
     * Lower latency interfaces are preferred over higher latency channels
     */
    pub round_trip_time: Duration,

    /**
     * Bandwidth in bytes per second
     */
    pub max_bandwidth: u32,

    /**
     * If true, the interface does support continuous connections.
     */
    pub continuous_connection: bool,
    /**
     * If true, the interface can be used to redirect DATEX messages to other endpoints
     * which are not directly connected to the interface (default: true)
     * Currently only enforced for broadcast messages
     */
    pub allow_redirects: bool,

    /**
     * If true, the interface is a secure channel (can not be eavesdropped).
     * This might be an already encrypted channel such as WebRTC or a channel
     * that is end-to-end and not interceptable by third parties
     */
    pub is_secure_channel: bool,
}
impl Default for InterfaceProperties {
    fn default() -> Self {
        InterfaceProperties {
            channel: "".to_string(),
            name: None,
            direction: InterfaceDirection::IN_OUT,
            reconnect_interval: None,
            round_trip_time: Duration::from_millis(0),
            max_bandwidth: u32::MAX,
            continuous_connection: false,
            allow_redirects: true,
            is_secure_channel: false,
        }
    }
}

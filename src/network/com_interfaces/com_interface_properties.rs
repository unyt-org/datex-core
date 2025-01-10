#[derive(PartialEq)]
pub enum InterfaceDirection {
    IN,
    OUT,
    IN_OUT,
}

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
    pub reconnect_interval: Option<u32>,
    /**
     * Estimated mean latency for this interface type in milliseconds (round trip time).
     * Lower latency interfaces are preferred over higher latency channels
     */
    pub latency: u32,
    /**
     * Bandwidth in bytes per second
     */
    pub bandwidth: u32,
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
}

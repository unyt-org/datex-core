use core::time::Duration;

#[derive(Debug)]
pub struct ComHubOptions {
    default_receive_timeout: Duration,
}

impl Default for ComHubOptions {
    fn default() -> Self {
        ComHubOptions {
            default_receive_timeout: Duration::from_secs(5),
        }
    }
}

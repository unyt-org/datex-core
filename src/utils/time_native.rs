use crate::stdlib::time::{SystemTime, UNIX_EPOCH};

use super::time::TimeTrait;
pub struct TimeNative;
impl TimeTrait for TimeNative {
    fn now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
    }
}

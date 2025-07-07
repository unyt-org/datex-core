use crate::runtime::global_context::get_global_context;

pub trait TimeTrait: Send + Sync {
    /// Returns the current time in milliseconds since the Unix epoch.
    fn now(&self) -> u64;
}
pub struct Time;
impl Time {
    pub fn now(&self) -> u64 {
        let time = get_global_context().time;
        let time = time.lock().unwrap();
        time.now()
    }
}

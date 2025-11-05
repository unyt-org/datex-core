use crate::runtime::global_context::get_global_context;
use core::marker::Sync;
use core::prelude::rust_2024::*;

pub trait TimeTrait: Send + Sync {
    /// Returns the current time in milliseconds since the Unix epoch.
    fn now(&self) -> u64;
}
pub struct Time;
impl Time {
    pub fn now() -> u64 {
        get_global_context().time.now()
    }
}

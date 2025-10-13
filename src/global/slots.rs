use num_enum::TryFromPrimitive;
use strum::Display;

/// internal slots address space, starting at 0xffffff_00
#[derive(
    Debug,
    Eq,
    PartialEq,
    TryFromPrimitive,
    Copy,
    Clone,
    Display,
    num_enum::IntoPrimitive,
)]
#[repr(u32)]
pub enum InternalSlot {
    ENDPOINT = 0xffffff00,
}

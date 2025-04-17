#![feature(coroutines)]
#![feature(iter_from_coroutine)]
#![allow(incomplete_features)]
#[macro_use]
extern crate mopa;

extern crate num_integer;

pub mod compiler;
pub mod crypto;
pub mod datex_values;
pub mod decompiler;
pub mod generator;
pub mod global;
pub mod logger;
pub mod network;
pub mod parser;
pub mod runtime;
pub mod tasks;
pub mod utils;

#[cfg(feature = "std")]
include!("./with_std.rs");

#[cfg(not(feature = "std"))]
include!("./without_std.rs");

pub mod stdlib {
    #[cfg(feature = "std")]
    pub use crate::with_std::*;
    #[cfg(not(feature = "std"))]
    pub use crate::without_std::*;
}

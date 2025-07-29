#![feature(coroutines)]
#![feature(iter_from_coroutine)]
#![feature(assert_matches)]
#![feature(gen_blocks)]
// FIXME #220 unused? Can be removed in the future.
// #![feature(type_alias_impl_trait)]
// #![feature(gen_blocks)]
#[macro_use]
extern crate mopa;

extern crate num_integer;

pub mod compiler;
pub mod crypto;
pub mod decompiler;
pub mod generator;
pub mod global;
pub mod logger;
pub mod network;
pub mod parser;
pub mod runtime;
pub mod task;
pub mod utils;
pub mod values;

// reexport macros
pub use datex_macros as macros;
extern crate self as datex_core;

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

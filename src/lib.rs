#![feature(coroutines)]
#![feature(iter_from_coroutine)]
#![feature(assert_matches)]
#![feature(gen_blocks)]
#![feature(let_chains)]
// #![allow(unused_parens)]
#![feature(async_iterator)]
#![feature(type_alias_impl_trait)]
#![feature(trait_alias)]
#![feature(box_patterns)]
#![feature(buf_read_has_data_left)]
// FIXME #228: remove in the future, not required in edition 2024, but RustRover complains
#![allow(unused_parens)]

#[macro_use]
extern crate mopa;

extern crate num_integer;

pub mod ast;
pub mod compiler;
pub mod crypto;
pub mod decompiler;
pub mod dif;
pub mod generator;
pub mod global;
mod libs;
pub mod logger;
pub mod network;
pub mod parser;
pub mod r#ref;
pub mod runtime;
pub mod task;
pub mod types;
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

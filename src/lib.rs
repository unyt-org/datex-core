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
#![feature(if_let_guard)]
#![feature(try_trait_v2)]
// FIXME #228: remove in the future, not required in edition 2024, but RustRover complains
#![allow(unused_parens)]
#![feature(associated_type_defaults)]
#![feature(core_float_math)]
#![feature(thread_local)]
#![allow(static_mut_refs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate mopa;

extern crate num_integer;

pub mod crypto;
pub mod dif;

#[cfg(feature = "compiler")]
pub mod ast;
#[cfg(feature = "compiler")]
pub mod compiler;
#[cfg(feature = "compiler")]
pub mod decompiler;
#[cfg(feature = "compiler")]
pub mod fmt;
pub mod generator;
pub mod global;
pub mod libs;
pub mod logger;
pub mod network;
pub mod parser;
pub mod references;
pub mod runtime;
#[cfg(feature = "compiler")]
pub mod visitor;

pub mod core_compiler;
pub mod serde;
pub mod task;
pub mod traits;
pub mod types;
pub mod utils;
pub mod values;

// reexport macros
pub use datex_macros as macros;
extern crate core;
extern crate self as datex_core;

pub mod stdlib {
    #[cfg(not(feature = "std"))]
    pub use nostd::{
        any, borrow, boxed, cell, collections, fmt, format, future, hash, io,
        ops, panic, pin, rc, string, sync, vec,
    };
    #[cfg(feature = "std")]
    pub use std::*;
}

pub mod std_sync {
    #[cfg(not(feature = "std"))]
    pub use spin::Mutex;
    #[cfg(feature = "std")]
    pub use std::sync::Mutex;
}

pub mod std_random {
    #[cfg(not(feature = "std"))]
    pub use foldhash::fast::RandomState;
    #[cfg(feature = "std")]
    pub use std::hash::RandomState;
}

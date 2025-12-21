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
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod network;
pub mod parser;
pub mod references;
pub mod runtime;
#[cfg(feature = "compiler")]
pub mod type_inference;
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
        ops, panic, pin, rc, string, sync, time, vec,
    };

    #[cfg(feature = "std")]
    pub use std::*;
}

// Note: always use collections mod for HashMap and HashSet
pub mod collections {
    #[cfg(not(feature = "std"))]
    pub use hashbrown::hash_map;
    #[cfg(not(feature = "std"))]
    pub use hashbrown::hash_map::HashMap;
    #[cfg(not(feature = "std"))]
    pub use hashbrown::hash_set;
    #[cfg(not(feature = "std"))]
    pub use hashbrown::hash_set::HashSet;
    #[cfg(feature = "std")]
    pub use std::collections::*;
}

pub mod std_sync {
    #[cfg(not(feature = "std"))]
    pub use spin::Mutex;
    #[cfg(feature = "std")]
    pub use std::sync::Mutex;
}

pub mod time {
    #[cfg(target_arch = "wasm32")]
    pub use web_time::*;
    #[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
    pub use embedded_time::*;
    #[cfg(all(not(target_arch = "wasm32"), feature = "std"))]
    pub use std::time::*;
}
pub mod std_random {
    #[cfg(not(feature = "std"))]
    pub use foldhash::fast::RandomState;
    #[cfg(feature = "std")]
    pub use std::hash::RandomState;
}

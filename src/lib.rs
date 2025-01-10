#![feature(coroutines)]
#![feature(iter_from_coroutine)]
// #![feature(anonymous_lifetime_in_impl_trait)]

#[macro_use]
extern crate mopa;

extern crate num_integer;

pub mod compiler;
pub mod datex_values;
pub mod decompiler;
pub mod generator;
pub mod global;
pub mod network;
pub mod parser;
pub mod runtime;
pub mod utils;

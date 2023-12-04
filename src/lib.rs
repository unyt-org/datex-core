#![feature(generator_trait)]
#![feature(generators)]
// #![feature(anonymous_lifetime_in_impl_trait)]

#[macro_use]
extern crate mopa;

extern crate num_integer;

pub mod compiler;
pub mod decompiler;
pub mod runtime;
pub mod parser;
pub mod global;
pub mod utils;
pub mod datex_values;
pub mod generator;
pub mod network;
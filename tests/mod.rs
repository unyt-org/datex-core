#![feature(assert_matches)]
#![feature(iter_from_coroutine)]
#![feature(coroutines)]
#![feature(thread_local)]
#![feature(box_patterns)]
#![allow(static_mut_refs)]
extern crate core;

pub mod context;
pub mod network;
pub mod values;

pub mod dif;
pub mod json;
pub mod parser;

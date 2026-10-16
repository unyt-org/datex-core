//! The `datex-core` crate is the core library for the DATEX system.
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(assert_matches)]
#![feature(gen_blocks)]
#![feature(async_iterator)]
#![feature(type_alias_impl_trait)]
#![feature(trait_alias)]
#![feature(deref_patterns)]
#![feature(if_let_guard)]
#![feature(try_trait_v2)]
#![feature(negative_impls, with_negative_coherence)]
// #![allow(unused_parens)]
#![feature(associated_type_defaults)]
#![feature(core_float_math)]
#![feature(thread_local)]
#![feature(future_join)]
#![allow(static_mut_refs)]
#![feature(variant_count)]
#![feature(const_cmp)]
#![feature(const_trait_impl)]
#![feature(custom_test_frameworks)]
#![feature(specialization)]
#![feature(const_default)]
#![feature(custom_inner_attributes)]
#![feature(decl_macro)]
#![feature(try_blocks)]
#![rustfmt::skip::macros(assert_regular_instructions_equal)]
#![feature(arbitrary_self_types)]
#![feature(vec_try_remove)]
extern crate alloc;
extern crate num_integer;

#[doc(hidden)]
#[macro_use]
mod preludes;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

#[cfg(feature = "std")]
extern crate std;

pub mod channel;
pub mod prelude;

#[cfg(feature = "ast")]
pub mod ast;
#[cfg(feature = "compiler")]
pub mod compiler;
pub mod decompiler;
#[cfg(feature = "compiler")]
pub mod fmt;
pub mod global;
pub mod instruction;
pub mod libs;
#[cfg(all(feature = "lsp", feature = "std"))]
pub mod lsp;
pub mod network;
#[cfg(feature = "parser")]
pub mod parser;
pub mod runtime;
pub mod shared_values;
#[cfg(feature = "compiler")]
pub mod type_inference;
#[cfg(feature = "compiler")]
pub mod visitor;

pub mod core_compiler;
pub mod dif;
pub mod disassembler;
pub mod dxb_parser;
mod stub;
pub mod task;
pub mod traits;
pub mod types;
pub mod utils;
pub mod value_updates;
pub mod values;

// reexport macros
pub use datex_macros_internal as macros;
extern crate core;

pub mod datex_registry;
pub mod inspector;
pub mod serde_compat;
pub mod shared_wrappers;

pub use inventory;

pub use serde;

/// HashMap and HashSet that work in both std and no_std environments.
pub mod collections {
    cfg_if::cfg_if! {
        if #[cfg(feature = "std")] {
            pub use std::collections::{HashMap, HashSet, hash_map, hash_set};
            pub use std::collections::hash_map::DefaultHasher;
            pub fn default_hasher() -> DefaultHasher {
                DefaultHasher::new()
            }
        } else {
            use core::hash::BuildHasher;
            pub use hashbrown::{HashMap, HashSet, hash_map, hash_set, DefaultHashBuilder, DefaultHasher};
            pub fn default_hasher() -> DefaultHasher {
                DefaultHashBuilder::default().build_hasher()
            }
        }
    }
}

/// Reexport of Mutex that works in both std and no_std environments.
pub mod std_sync {
    cfg_if::cfg_if! {
        if #[cfg(feature = "std")] {
            pub use std::sync::Mutex;
        } else {
            pub use spin::Mutex;
        }
    }
}

/// Crypto implementations selection based on target architecture and features.
pub mod crypto {
    cfg_if::cfg_if! {
        if #[cfg(any(feature = "target_native", test))] {
            pub use datex_crypto_native::CryptoNative as CryptoImpl;
        } else if #[cfg(feature = "target_esp_shared")] {
            pub use datex_crypto_esp32::CryptoEsp32 as CryptoImpl;
        } else if #[cfg(feature = "target_wasm")] {
            pub use datex_crypto_web::CryptoWeb as CryptoImpl;
        } else {
            pub use crate::stub::crypto::CryptoStub as CryptoImpl;
        }
    }
}

pub mod time {
    mod system_time {
        cfg_if::cfg_if! {
            if #[cfg(feature = "target_wasm")] {
                pub use web_time::{SystemTime, UNIX_EPOCH};
            } else if #[cfg(feature = "target_native")] {
                pub use std::time::{SystemTime, UNIX_EPOCH};
            }
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "target_wasm")] {
            pub use web_time::{Instant};
        } else if #[cfg(feature = "std")] {
            pub use std::time::{Instant};
        } else if #[cfg(feature = "embassy_runtime")] {
            pub use embassy_time::{Instant};
        } else {
            pub use crate::stub::time::{Instant};
        }
    }

    // current unix timestamp in milliseconds
    pub fn now_ms() -> u64 {
        cfg_if::cfg_if! {
            if #[cfg(any(feature = "target_wasm", feature = "target_native"))] {
                use system_time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("System time is before UNIX_EPOCH")
                    .as_millis() as u64
            } else if #[cfg(feature = "target_esp_shared")] {
                datex_crypto_esp32::now_ms()
            } else {
                Instant::now().elapsed().as_millis() as u64
            }
        }
    }
}

pub mod random {
    cfg_if::cfg_if! {
        if #[cfg(feature = "std")] {
            pub use std::hash::RandomState;
        } else {
            pub use foldhash::fast::RandomState;
        }
    }
}

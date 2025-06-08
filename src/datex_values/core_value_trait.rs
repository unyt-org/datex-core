use std::fmt::Display;

use crate::datex_values::traits::soft_eq::SoftEq;

pub trait CoreValueTrait: Display + Send + Sync + SoftEq {}

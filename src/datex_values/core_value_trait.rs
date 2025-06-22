use std::fmt::Display;

use crate::datex_values::traits::structural_eq::StructuralEq;

pub trait CoreValueTrait: Display + StructuralEq {}

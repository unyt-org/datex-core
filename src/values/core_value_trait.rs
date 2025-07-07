use std::fmt::Display;

use crate::values::traits::structural_eq::StructuralEq;

pub trait CoreValueTrait: Display + StructuralEq {}

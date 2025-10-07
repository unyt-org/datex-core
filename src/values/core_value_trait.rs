use std::fmt::Display;

use crate::traits::structural_eq::StructuralEq;

pub trait CoreValueTrait: Display + StructuralEq {}

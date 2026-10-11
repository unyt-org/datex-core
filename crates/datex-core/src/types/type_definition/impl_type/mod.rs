use crate::{prelude::*, shared_values::PointerAddress};
use core::fmt::Display;
use itertools::Itertools;

pub mod serde_dif;
mod type_match;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplMarkers {
    pub impl_markers: Vec<PointerAddress>,
}

impl ImplMarkers {
    pub fn new(impl_markers: Vec<PointerAddress>) -> Self {
        Self { impl_markers }
    }
}

impl Display for ImplMarkers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.impl_markers.iter().join(" + "))
    }
}

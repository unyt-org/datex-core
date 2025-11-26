use crate::dif::r#type::DIFTypeContainer;
use crate::references::reference::mutability_as_int;
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::memory::Memory;
use core::cell::RefCell;
use core::prelude::rust_2024::*;
use datex_core::dif::value::DIFValueContainer;
use serde::{Deserialize, Serialize};
use datex_core::dif::r#type::DIFTypeDefinition;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFReference {
    pub value: DIFValueContainer,
    pub allowed_type: DIFTypeDefinition,
    #[serde(rename = "mut")]
    #[serde(with = "mutability_as_int")]
    pub mutability: ReferenceMutability,
}

impl DIFReference {
    pub fn from_reference(
        reference: &Reference,
        memory: &RefCell<Memory>,
    ) -> Self {
        let value = DIFValueContainer::from_value_container(
            &reference.value_container(),
            memory,
        );
        let allowed_type = DIFTypeDefinition::from_type_container(
            &reference.allowed_type(),
            memory,
        );
        DIFReference {
            value,
            allowed_type,
            mutability: reference.mutability(),
        }
    }
}

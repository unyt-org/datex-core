use crate::dif::r#type::DIFTypeContainer;
use crate::references::reference::mutability_as_int;
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::memory::Memory;
use datex_core::dif::value::DIFValueContainer;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFReference {
    pub value: DIFValueContainer,
    pub allowed_type: DIFTypeContainer,
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
        let allowed_type = DIFTypeContainer::from_type_container(
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

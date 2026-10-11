use crate::{
    libs::core::{type_id::CoreLibTypeId, value_id::CoreLibValueId},
    prelude::*,
};
use binrw::{BinRead, BinWrite};
use core::{fmt::Display, ops::Deref, str::FromStr};
use serde::Serialize;

pub const TYPE_SPACE_BASE: u16 = 1;
pub const TYPE_VARIANT_SPACE_BASE: u16 = 500;
pub const VALUE_SPACE_BASE: u16 = 1000;

#[derive(
    BinWrite, BinRead, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize,
)]
#[serde(transparent)]
#[brw(little)]
pub struct CoreLibIdIndex(pub u16);
impl CoreLibIdIndex {
    pub fn new(index: u16) -> Self {
        Self(index)
    }
}
impl Display for CoreLibIdIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for CoreLibIdIndex {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub trait CoreLibIdTrait:
    TryFrom<CoreLibIdIndex> + Into<CoreLibIdIndex>
{
    fn name(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum CoreLibId {
    CoreMap,
    Type(CoreLibTypeId),
    Value(CoreLibValueId),
}

impl CoreLibId {
    pub fn try_from_str(string: &str) -> Option<Self> {
        if string == "core" {
            return Some(CoreLibId::CoreMap);
        }
        CoreLibTypeId::try_from_str(string)
            .map(CoreLibId::Type)
            .or_else(|| {
                CoreLibValueId::from_str(string).map(CoreLibId::Value).ok()
            })
    }
}

impl From<CoreLibTypeId> for CoreLibId {
    fn from(type_id: CoreLibTypeId) -> Self {
        CoreLibId::Type(type_id)
    }
}
impl From<CoreLibValueId> for CoreLibId {
    fn from(value_id: CoreLibValueId) -> Self {
        CoreLibId::Value(value_id)
    }
}

impl Display for CoreLibId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CoreLibId::Type(type_id) => write!(f, "{}", type_id),
            CoreLibId::Value(value_id) => write!(f, "{}", value_id),
            CoreLibId::CoreMap => write!(f, "Map"),
        }
    }
}

impl From<CoreLibId> for CoreLibIdIndex {
    fn from(val: CoreLibId) -> Self {
        match val {
            CoreLibId::CoreMap => CoreLibIdIndex(0),
            CoreLibId::Type(type_id) => type_id.into(),
            CoreLibId::Value(value_id) => value_id.into(),
        }
    }
}

impl TryFrom<CoreLibIdIndex> for CoreLibId {
    type Error = ();

    fn try_from(bytes: CoreLibIdIndex) -> Result<Self, Self::Error> {
        if bytes.0 == 0 {
            Ok(CoreLibId::CoreMap)
        } else if let Ok(type_id) = CoreLibTypeId::try_from(bytes) {
            Ok(CoreLibId::Type(type_id))
        } else if let Ok(value_id) = CoreLibValueId::try_from(bytes) {
            Ok(CoreLibId::Value(value_id))
        } else {
            Err(())
        }
    }
}

impl CoreLibIdTrait for CoreLibId {
    fn name(&self) -> String {
        match self {
            CoreLibId::Type(type_id) => type_id.name(),
            CoreLibId::Value(value_id) => value_id.name(),
            CoreLibId::CoreMap => "core".to_string(),
        }
    }
}

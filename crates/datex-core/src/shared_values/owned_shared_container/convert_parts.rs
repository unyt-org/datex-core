use crate::shared_values::OwnedSharedContainer;
use crate::traits::convert_parts::{FromParts, HasPartsKind};

impl HasPartsKind for OwnedSharedContainer {}
impl FromParts for OwnedSharedContainer {}
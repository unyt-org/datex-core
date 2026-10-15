mod serde_dif;

use crate::{
    prelude::*,
    shared_values::base_shared_value_container::observers::TransceiverId,
    values::value_container::{ValueContainer},
};
use strum::AsRefStr;
mod append_entry;
mod decrement;
mod delete_entry;
mod increment;
mod list_splice;
mod replace;
mod set_entry;

pub use append_entry::*;
pub use decrement::*;
pub use delete_entry::*;
pub use increment::*;
pub use list_splice::*;
pub use replace::*;
pub use set_entry::*;

#[derive(Clone, Debug, PartialEq, AsRefStr, Hash)]
#[strum(serialize_all = "snake_case")]
pub enum UpdateOperation {
    /// Represents a replacement operation for a value.
    Replace(Box<ReplaceUpdateData>),

    /// Represents an update to a specific property of a value.
    /// The `key` specifies which property to update, and `value` is the new value for that property.
    SetEntry(Box<SetEntryUpdateData>),

    /// Represents the removal of a specific property from a value.
    DeleteEntry(Box<DeleteEntryUpdateData>),

    /// Represents clearing all elements from a collection-type value (like an array or map).
    Clear,

    /// Represents adding a new element to a collection-type value (like an array or map).
    AppendEntry(Box<AppendEntryUpdateData>),

    /// Special update operation for list values that allows splicing
    ListSplice(Box<ListSpliceUpdateData>),

    /// Increment operation for numeric values
    Increment(Box<IncrementUpdateData>),

    /// Decrement operation for numeric values
    Decrement(Box<DecrementUpdateData>),
}
impl UpdateOperation {
    pub fn replace(value: ValueContainer) -> Self {
        UpdateOperation::Replace(Box::new(ReplaceUpdateData::new(value)))
    }
    pub fn set_entry(key: ValueContainer, value: ValueContainer) -> Self {
        UpdateOperation::SetEntry(Box::new(SetEntryUpdateData::new(key, value)))
    }
    pub fn delete_entry(key: ValueContainer) -> Self {
        UpdateOperation::DeleteEntry(Box::new(DeleteEntryUpdateData::new(key)))
    }
    pub fn clear() -> Self {
        UpdateOperation::Clear
    }
    pub fn append_entry(value: ValueContainer) -> Self {
        UpdateOperation::AppendEntry(Box::new(AppendEntryUpdateData::new(
            value,
        )))
    }
    pub fn list_splice(
        start_index: u32,
        delete_count: u32,
        items_to_insert: Vec<ValueContainer>,
    ) -> Self {
        UpdateOperation::ListSplice(Box::new(ListSpliceUpdateData::new(
            start_index,
            delete_count,
            items_to_insert,
        )))
    }
    pub fn increment(amount: ValueContainer) -> Self {
        UpdateOperation::Increment(Box::new(IncrementUpdateData::new(amount)))
    }
    pub fn decrement(amount: ValueContainer) -> Self {
        UpdateOperation::Decrement(Box::new(DecrementUpdateData::new(amount)))
    }
}

#[derive(Clone, Debug, PartialEq, AsRefStr, Hash)]
pub enum UpdateOperator {
    AppendEntry, // Set<5> += 4
    DeleteEntry, // Set<5, 4> -= 4
    Increment,   // 5 += 1
    Decrement,   // 5 -= 1
}

impl UpdateOperation {
    /// Creates a new [Update] struct with the given [TransceiverId] as source id.
    pub fn with_source(
        self,
        source_id: TransceiverId,
        path: Vec<ValueContainer>,
    ) -> Update {
        Update {
            source_id,
            data: UpdateData::new_with_path(self, path),
        }
    }

    /// Creates a new [Update] struct with the given [TransceiverId] as source id and an empty path.
    pub fn with_source_root(self, source_id: TransceiverId) -> Update {
        Self::with_source(self, source_id, vec![])
    }
}
/// Represents an update to a value from a source [TransceiverId]
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Update {
    source_id: TransceiverId,
    data: UpdateData,
}
impl Update {
    /// Creates a new [Update] struct with the given [TransceiverId] as source id and an empty path.
    pub fn new(source_id: TransceiverId, data: UpdateData) -> Self {
        Update { source_id, data }
    }
    pub fn source_id(&self) -> &TransceiverId {
        &self.source_id
    }
    pub fn data(&self) -> &UpdateData {
        &self.data
    }
    pub fn path(&self) -> &[ValueContainer] {
        self.data.path()
    }
    pub fn operation(&self) -> &UpdateOperation {
        self.data.operation()
    }
    pub fn into_operation(self) -> UpdateOperation {
        self.data.into_operation()
    }
    pub fn into_parts(self) -> (TransceiverId, UpdateOperation, Vec<ValueContainer>) {
        let (operation, path) = self.data.into_parts();
        (self.source_id, operation, path)
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct UpdateData {
    // The actual update operation, which can be one of several types of updates (e.g., replace, set entry, delete entry, etc.)
    operation: UpdateOperation,
    /// Path to the value being updated (e.g. in case of nested property access)
    /// If the vector is empty, the update is applied to the root value.
    path: Vec<ValueContainer>,
}

impl UpdateData {
    /// Creates a new [UpdateData] struct with the given [UpdateOperation] and an empty path.
    pub fn new(operation: UpdateOperation) -> Self {
        UpdateData {
            operation,
            path: vec![],
        }
    }
    pub fn new_with_path(
        operation: UpdateOperation,
        path: Vec<ValueContainer>,
    ) -> Self {
        UpdateData { operation, path }
    }

    /// The path to the value being updated. If the vector is empty, the update is applied to the root value.
    pub fn path(&self) -> &[ValueContainer] {
        &self.path
    }

    pub fn into_parts(self) -> (UpdateOperation, Vec<ValueContainer>) {
        (self.operation, self.path)
    }

    pub fn operation(&self) -> &UpdateOperation {
        &self.operation
    }
    pub fn into_operation(self) -> UpdateOperation {
        self.operation
    }
}

//! DIF is a representation of DATEX values and updates that can be serialized and deserialized in a compact format to act
//! as interface for communication with outside components or languages.
pub mod deserialize_serde_context;
pub mod deserialize_with_serde_context;
pub mod dif_interface;
pub mod error;
pub mod pointer_address;
pub mod serde_context;
pub mod serialize_with_serde_context;
pub mod value_with_serde_context;

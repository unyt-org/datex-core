// use serde::{Deserialize, Serialize, Serializer, Deserializer};
// use crate::collections::HashMap;
// use core::result::Result;
// use crate::stdlib::vec::Vec;
// use core::option::Option;
// use core::hash::Hash;
// use core::cmp::Eq;
//
// pub fn serialize_map<S, K: Serialize, V: Serialize>(map: &HashMap<K,V>, serializer: S) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     let vec: Vec<_> = map.iter().collect();
//     vec.serialize(serializer)
// }
//
// pub fn serialize_map_option<S, K: Serialize, V: Serialize>(map: &Option<HashMap<K, V>>, serializer: S) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     if let Some(map) = map {
//         let vec: Vec<_> = map.iter().collect();
//         vec.serialize(serializer)
//     }
//     else {
//         serializer.serialize_none()
//     }
// }
//
// pub fn deserialize_map<'de, D, K: Deserialize<'de> + Eq + Hash, V: Deserialize<'de>>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let vec: Vec<_> = Vec::deserialize(deserializer)?;
//     Ok(vec.into_iter().collect())
// }
//
// pub fn deserialize_map_option<'de, D, K: Deserialize<'de> + Eq + Hash, V: Deserialize<'de>>(
//     deserializer: D,
// ) -> Result<Option<HashMap<K, V>>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let opt: Option<Vec<_>> = Option::deserialize(deserializer)?;
//     Ok(opt.map(|v| serde::de::value::MapDeserializer::new(v.into_iter()).deserialize_map(deserializer)))
// }
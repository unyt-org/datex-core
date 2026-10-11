use crate::{
    random::RandomState, traits::classification::Classification,
    values::core_values::native::DatexNativeBase,
};
use core::hash::Hash;
use indexmap::IndexMap;

impl<K, V> Classification for IndexMap<K, V, RandomState>
where
    K: DatexNativeBase + Eq + Hash + 'static,
    V: DatexNativeBase + 'static,
{
}

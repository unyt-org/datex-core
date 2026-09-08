use crate::{
    preludes::derive::DatexNative,
    random::RandomState,
    values::core_values::native::{DatexNativeBase, DatexNativeOps},
};
use core::{any::Any, hash::Hash};
use indexmap::IndexMap;

impl<K, V> DatexNative for IndexMap<K, V, RandomState>
where
    K: DatexNativeBase + Eq + Hash + 'static,
    V: DatexNativeBase + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl<K, V> DatexNativeOps for IndexMap<K, V, RandomState>
where
    K: DatexNativeBase + Eq + Hash + 'static,
    V: DatexNativeBase + 'static,
{
}

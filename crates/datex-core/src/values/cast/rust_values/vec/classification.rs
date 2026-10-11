use crate::{
    prelude::*, traits::classification::Classification,
    values::core_values::native::DatexNativeBase,
};

impl<T> Classification for Vec<T> where T: DatexNativeBase + 'static {}

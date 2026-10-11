use crate::{
    traits::{
        datex_native_only_structural::DatexNativeOnlyStructural,
        datex_native_structural::DatexNativeStructural,
    },
    values::core_values::instant::Instant,
};

impl DatexNativeStructural for Instant {}
impl DatexNativeOnlyStructural for Instant {}

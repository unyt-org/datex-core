use crate::values::core_values::text::Text;
use core::hash::{Hash, Hasher};

impl Hash for Text {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}
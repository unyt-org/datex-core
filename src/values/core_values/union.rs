use serde::{Deserialize, Serialize};

use crate::values::{
    core_value_trait::CoreValueTrait, core_values::r#type::r#type::Type,
    datex_type::CoreValueType, traits::structural_eq::StructuralEq,
    value_container::ValueContainer,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Union {
    pub options: Vec<ValueContainer>,
}
fn is_instance_of(value: &ValueContainer, parent: &ValueContainer) -> bool {
    if parent.is_type() && !value.is_type() {
        let value_type = value.r#type();
        let parent_type = parent.to_value().borrow().cast_to_type().unwrap();
        return value_type.is_typeof(&parent_type);
    } else if parent.is_type() && value.is_type() {
        let value_type = value.to_value().borrow().cast_to_type().unwrap();
        let parent_type = parent.to_value().borrow().cast_to_type().unwrap();
        return value_type.is_typeof(&parent_type);
    }
    false
}

fn normalize_union(options: &mut Vec<ValueContainer>) {
    options.dedup_by(|a, b| a == b);

    let mut keep = Vec::new();
    for i in 0..options.len() {
        let v = &options[i];
        if !v.is_type() {
            // literal → check if some Type in options covers it
            if options.iter().any(|o| o.is_type() && is_instance_of(v, o)) {
                continue; // skip this literal, it's subsumed
            }
        }
        keep.push(v.clone());
    }

    *options = keep;
}

impl Union {
    pub fn new(options: Vec<impl Into<ValueContainer>>) -> Self {
        let mut options =
            options.into_iter().map(Into::into).collect::<Vec<_>>();
        normalize_union(&mut options);
        if options.is_empty() {
            panic!("Union must have at least one option");
        }
        Union { options }
    }

    pub fn matches_ref(&self, v: &ValueContainer) -> bool {
        self.matches(v.clone())
    }
    pub fn matches(&self, v: impl Into<ValueContainer>) -> bool {
        let v = v.into();
        self.options.iter().any(|opt| {
            if opt.is_type() {
                is_instance_of(&v, opt)
            } else {
                opt == &v
            }
        })
    }

    pub fn as_options(&self) -> &[ValueContainer] {
        &self.options
    }

    pub fn len(&self) -> usize {
        self.options.len()
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }
}
impl CoreValueTrait for Union {}

impl std::fmt::Display for Union {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let options = self
            .options
            .iter()
            .map(|opt| opt.to_string())
            .collect::<Vec<_>>()
            .join(" | ");
        write!(f, "{}", options)
    }
}

impl StructuralEq for Union {
    fn structural_eq(&self, other: &Self) -> bool {
        self.options == other.options
    }
}

impl From<Vec<ValueContainer>> for Union {
    fn from(options: Vec<ValueContainer>) -> Self {
        Union::new(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        datex_array,
        values::{
            core_value::CoreValue,
            core_values::{
                integer::integer::Integer,
                text::Text,
                r#type::{core::integer, r#type::Type},
            },
            value_container::ValueContainer,
        },
    };

    #[test]
    fn test_type_and_value() {
        let union = Union::new(vec![
            ValueContainer::from(Integer::from(1)),
            ValueContainer::from(integer()),
        ]);

        assert_eq!(union.len(), 1);

        assert!(union.matches(Integer::from(1)));
        assert!(union.matches(Integer::from(2)));
        assert!(union.matches(Integer::from(42)));

        assert!(!union.matches(Text::from("test")));
        assert!(!union.matches(Text::from("test2")));
    }

    #[test]
    fn test_union_creation() {
        let union = Union::new(
            datex_array![Integer::from(1), Text::from("test")].into(),
        );
        assert_eq!(union.len(), 2);

        let union =
            Union::new(vec![ValueContainer::from(1), ValueContainer::from(2)]);
        assert_eq!(union.len(), 2);
    }

    #[test]
    fn test_duplicate_union_options() {
        let union = Union::new(vec![
            ValueContainer::from(1),
            ValueContainer::from(1),
            ValueContainer::from(2),
        ]);
        assert_eq!(union.len(), 2);
        assert!(union.matches(ValueContainer::from(1)));
        assert!(union.matches(ValueContainer::from(2)));
    }

    #[test]
    fn test_union_matches() {
        let union = Union::new(
            datex_array![Integer::from(1), Text::from("test")].into(),
        );

        // Positive tests
        assert!(union.matches(Integer::from(1)));
        assert!(union.matches(Text::from("test")));

        // Negative tests
        assert!(!union.matches(Integer::from(2)));
        assert!(!union.matches(Text::from("test2")));

        // Reference tests
        let union =
            Union::new(vec![ValueContainer::from(1), ValueContainer::from(2)]);
        assert!(union.matches_ref(&ValueContainer::from(1)));
        assert!(!union.matches_ref(&ValueContainer::from(3)));
    }

    #[test]
    fn test_union_display() {
        let union =
            Union::new(vec![ValueContainer::from(1), ValueContainer::from(2)]);
        assert_eq!(union.to_string(), "1 | 2");
    }
}

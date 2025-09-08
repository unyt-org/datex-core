use serde::{Deserialize, Serialize};

use crate::values::{
    core_value_trait::CoreValueTrait, core_values::r#type::Type,
    traits::structural_eq::StructuralEq, value_container::ValueContainer,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Union {
    pub options: Vec<Type>,
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
                TypeNew::value_matches(&v, opt)
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
            core_values::{
                integer::{integer::Integer, typed_integer::TypedInteger},
                text::Text,
                r#type::core::integer,
            },
            value_container::ValueContainer,
        },
    };

    #[test]
    fn union_creation() {
        let union = Union::new(
            datex_list![Integer::from(1), Text::from("test")].into(),
        );
        assert_eq!(union.len(), 2);

        let union =
            Union::new(vec![ValueContainer::from(1), ValueContainer::from(2)]);
        assert_eq!(union.len(), 2);
    }

    #[test]
    fn duplicate_union_options() {
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
    fn union_matches() {
        let union = Union::new(
            datex_list![Integer::from(1), Text::from("test")].into(),
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
    fn union_display() {
        let union =
            Union::new(vec![ValueContainer::from(1), ValueContainer::from(2)]);
        assert_eq!(union.to_string(), "1 | 2");
    }

    #[test]
    fn integer_and_typed_integer() {
        let union = Union::new(
            datex_list![TypedInteger::from(1u8), Integer::from(1)].into(),
        );

        assert_eq!(union.len(), 2);
        assert_eq!(union.to_string(), "1 | 1");

        assert!(union.matches(Integer::from(1)));
        assert!(union.matches(TypedInteger::from(1u8)));
        assert!(!union.matches(Integer::from(42)));
        assert!(!union.matches(TypedInteger::from(1u16)));
    }

    #[test]
    fn typed() {
        let union = Union::new(datex_list![integer()].into());

        assert_eq!(union.len(), 1);
        assert_eq!(union.to_string(), "integer");

        assert!(union.matches(Integer::from(1)));
        assert!(union.matches(Integer::from(2)));
        assert!(union.matches(Integer::from(42)));
    }

    #[test]
    fn type_and_value() {
        let union =
            Union::new(datex_list![Integer::from(1), integer()].into());

        assert_eq!(union.len(), 1);

        assert!(union.matches(Integer::from(1)));
        assert!(union.matches(Integer::from(2)));
        assert!(union.matches(Integer::from(42)));
    }
}

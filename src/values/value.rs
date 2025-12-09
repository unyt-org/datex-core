use crate::libs::core::CoreLibPointerId;
use crate::references::type_reference::TypeReference;
use crate::stdlib::boxed::Box;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::value_container::ValueError;
use core::fmt::{Display, Formatter};
use core::ops::{Add, AddAssign, Deref, Neg, Not, Sub};
use core::prelude::rust_2024::*;
use core::result::Result;
use log::error;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Value {
    pub inner: CoreValue,
    pub actual_type: Box<TypeDefinition>,
}

/// Two values are structurally equal, if their inner values are structurally equal, regardless
/// of the actual_type of the values
impl StructuralEq for Value {
    fn structural_eq(&self, other: &Self) -> bool {
        self.inner.structural_eq(&other.inner)
    }
}

/// Value equality corresponds to partial equality:
/// Both type and inner value are the same
impl ValueEq for Value {
    fn value_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Deref for Value {
    type Target = CoreValue;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Into<CoreValue>> From<T> for Value {
    fn from(inner: T) -> Self {
        let inner = inner.into();
        let new_type = inner.default_type_definition();
        Value {
            inner,
            actual_type: Box::new(new_type),
        }
    }
}
impl Value {
    pub fn null() -> Self {
        CoreValue::Null.into()
    }
}

impl Value {
    pub fn is_type(&self) -> bool {
        core::matches!(self.inner, CoreValue::Type(_))
    }
    pub fn is_null(&self) -> bool {
        core::matches!(self.inner, CoreValue::Null)
    }
    pub fn is_text(&self) -> bool {
        core::matches!(self.inner, CoreValue::Text(_))
    }
    pub fn is_integer_i8(&self) -> bool {
        core::matches!(
            &self.inner,
            CoreValue::TypedInteger(TypedInteger::I8(_))
        )
    }
    pub fn is_bool(&self) -> bool {
        core::matches!(self.inner, CoreValue::Boolean(_))
    }
    pub fn is_map(&self) -> bool {
        core::matches!(self.inner, CoreValue::Map(_))
    }
    pub fn is_list(&self) -> bool {
        core::matches!(self.inner, CoreValue::List(_))
    }
    pub fn actual_type(&self) -> &TypeDefinition {
        self.actual_type.as_ref()
    }

    /// Returns true if the current Value's actual type is the same as its default type
    /// E.g. if the type is integer for an Integer value, or integer/u8 for a typed integer value
    /// This will return false for an integer value if the actual type is one of the following:
    /// * an ImplType<integer, x>
    /// * a new nominal type containing an integer
    /// TODO: this does not match all cases of default types from the point of view of the compiler -
    /// integer variants (despite bigint) can be distinguished based on the instruction code, but for text variants,
    /// the variant must be included in the compiler output - so we need to handle theses cases as well.
    /// Generally speaking, all variants except the few integer variants should never be considered default types.
    pub fn has_default_type(&self) -> bool {
        if let TypeDefinition::Reference(type_reference) =
            self.actual_type.as_ref()
            && let TypeReference {
                pointer_address: Some(pointer_address),
                ..
            } = &*type_reference.borrow()
            && let Ok(actual_type_core_ptr_id) =
                CoreLibPointerId::try_from(pointer_address)
        {
            // actual_type has core type pointer id which is equal to the default core type pointer id of self.inner
            let self_default_type_ptr_id = CoreLibPointerId::from(&self.inner);
            self_default_type_ptr_id == actual_type_core_ptr_id
        } else {
            false
        }
    }
}

impl Add for Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: Value) -> Self::Output {
        Ok((&self.inner + &rhs.inner)?.into())
    }
}

impl Add for &Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: &Value) -> Self::Output {
        Value::add(self.clone(), rhs.clone())
    }
}

impl Sub for Value {
    type Output = Result<Value, ValueError>;
    fn sub(self, rhs: Value) -> Self::Output {
        Ok((&self.inner - &rhs.inner)?.into())
    }
}

impl Sub for &Value {
    type Output = Result<Value, ValueError>;
    fn sub(self, rhs: &Value) -> Self::Output {
        Value::sub(self.clone(), rhs.clone())
    }
}

impl Neg for Value {
    type Output = Result<Value, ValueError>;

    fn neg(self) -> Self::Output {
        (-self.inner).map(Value::from)
    }
}

impl Not for Value {
    type Output = Option<Value>;

    fn not(self) -> Self::Output {
        (!self.inner).map(Value::from)
    }
}

// TODO #119: crate a TryAddAssign trait etc.
impl<T> AddAssign<T> for Value
where
    Value: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        let rhs: Value = rhs.into();
        let res = self.inner.clone() + rhs.inner;
        if let Ok(res) = res {
            self.inner = res;
        } else {
            error!("Failed to add value: {res:?}");
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        core::write!(f, "{}", self.inner)
    }
}

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => Value::null(),
        }
    }
}

#[cfg(test)]
/// Tests for the Value struct and its methods.
/// This module contains unit tests for the Value struct, including its methods and operations.
/// The value is a holder for a combination of a CoreValue representation and its actual type.
mod tests {
    use super::*;
    use crate::{
        assert_structural_eq, datex_list,
        logger::init_logger_debug,
        values::core_values::{
            endpoint::Endpoint,
            integer::{Integer, typed_integer::TypedInteger},
            list::List,
        },
    };
    use core::str::FromStr;
    use datex_core::libs::core::{
        get_core_lib_type, get_core_lib_type_reference,
    };
    use log::info;

    #[test]
    fn endpoint() {
        init_logger_debug();
        let endpoint = Value::from(Endpoint::from_str("@test").unwrap());
        assert_eq!(endpoint.to_string(), "@test");
    }

    #[test]
    fn new_addition_assignments() {
        let mut x = Value::from(42i8);
        let y = Value::from(27i8);

        x += y.clone();
        assert_eq!(x, Value::from(69i8));
    }

    #[test]
    fn new_additions() {
        let x = Value::from(42i8);
        let y = Value::from(27i8);

        let z = (x.clone() + y.clone()).unwrap();
        assert_eq!(z, Value::from(69i8));
    }

    #[test]
    fn list() {
        init_logger_debug();
        let mut a = List::from(vec![
            Value::from("42"),
            Value::from(42),
            Value::from(true),
        ]);

        a.push(Value::from(42));
        a.push(4);

        assert_eq!(a.len(), 5);

        let b = List::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.len(), 11);

        let c = datex_list![1, "test", 3, true, false];
        assert_eq!(c.len(), 5);
        assert_eq!(c[0], 1.into());
        assert_eq!(c[1], "test".into());
        assert_eq!(c[2], 3.into());
    }

    #[test]
    fn boolean() {
        init_logger_debug();
        let a = Value::from(true);
        let b = Value::from(false);
        let c = Value::from(false);
        assert_ne!(a, b);
        assert_eq!(b, c);

        let d = (!b.clone()).unwrap();
        assert_eq!(a, d);

        // We can't add two booleans together, so this should return None
        let a_plus_b = a.clone() + b.clone();
        assert!(a_plus_b.is_err());
    }

    #[test]
    fn equality_same_type() {
        init_logger_debug();
        let a = Value::from(42i8);
        let b = Value::from(42i8);
        let c = Value::from(27i8);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);

        info!("{} === {}", a.clone(), b.clone());
        info!("{} !== {}", a.clone(), c.clone());
    }

    #[test]
    fn decimal() {
        init_logger_debug();
        let a = Value::from(42.1f32);
        let b = Value::from(27f32);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b, Value::from(69.1f32));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn null() {
        init_logger_debug();

        let null_value = Value::null();
        assert_eq!(null_value.to_string(), "null");

        let maybe_value: Option<i8> = None;
        let null_value = Value::from(maybe_value);
        assert_eq!(null_value.to_string(), "null");
        assert!(null_value.is_null());
    }

    #[test]
    fn addition() {
        init_logger_debug();
        let a = Value::from(42i8);
        let b = Value::from(27i8);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b, Value::from(69i8));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn string_concatenation() {
        init_logger_debug();
        let a = Value::from("Hello ");
        let b = Value::from(42i8);

        assert!(a.is_text());
        assert!(b.is_integer_i8());

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        let b_plus_a = (b.clone() + a.clone()).unwrap();

        assert!(a_plus_b.is_text());
        assert!(b_plus_a.is_text());

        assert_eq!(a_plus_b, Value::from("Hello 42"));
        assert_eq!(b_plus_a, Value::from("42Hello "));

        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
        info!("{} + {} = {}", b.clone(), a.clone(), b_plus_a);
    }

    #[test]
    fn structural_equality() {
        let a = Value::from(42_i8);
        let b = Value::from(42_i32);
        assert!(a.is_integer_i8());

        assert_structural_eq!(a, b);

        assert_structural_eq!(
            Value::from(TypedInteger::I8(42)),
            Value::from(TypedInteger::U32(42)),
        );

        assert_structural_eq!(
            Value::from(42_i8),
            Value::from(Integer::from(42_i8))
        );
    }

    #[test]
    fn default_types() {
        let val = Value::from(Integer::from(42));
        assert!(val.has_default_type());

        let val = Value::from(42i8);
        assert!(val.has_default_type());

        let val = Value {
            inner: CoreValue::Integer(Integer::from(42)),
            actual_type: Box::new(TypeDefinition::ImplType(
                Box::new(get_core_lib_type(CoreLibPointerId::Integer(None))),
                vec![],
            )),
        };

        assert!(!val.has_default_type());
    }
}

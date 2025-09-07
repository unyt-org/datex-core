use super::datex_type::CoreValueType;
use crate::values::core_value::CoreValue;
use crate::values::core_values::r#type::Type;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::type_container::TypeContainer;
use crate::values::value_container::ValueError;
use log::error;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Deref, Not, Sub};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Value {
    pub inner: CoreValue,
    pub actual_type: Box<TypeContainer>,
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
        let new_type = inner.get_default_type();

        Value {
            inner,
            actual_type: Box::new(new_type), // Box::new(Type::new(
                                             //     "core:fixme",
                                             //     TypeDescriptor::Core(actual_type),
                                             // )),
        }
    }
}

impl Value {
    pub fn is_type(&self) -> bool {
        matches!(self.inner, CoreValue::Type(_))
    }

    pub fn is_of_type(&self, target: CoreValueType) -> bool {
        self.get_type() == target
    }
    pub fn is_null(&self) -> bool {
        self.is_of_type(CoreValueType::Null)
    }
    pub fn is_text(&self) -> bool {
        self.is_of_type(CoreValueType::Text)
    }
    pub fn is_i8(&self) -> bool {
        self.is_of_type(CoreValueType::I8)
    }
    pub fn is_bool(&self) -> bool {
        self.is_of_type(CoreValueType::Boolean)
    }

    pub fn actual_type(&self) -> &TypeContainer {
        self.actual_type.as_ref()
    }

    /// Attempts to cast the value to the target type, returning an Option<Value>.
    /// If the cast fails, it returns None.
    /// This is useful for cases where you want to handle the failure gracefully.
    /// # Arguments
    /// * `target_type` - The target type to cast the value to.
    /// # Returns
    /// * `Option<Value>` - Some(Value) if the cast is successful, None if it fails.
    ////
    /// # Example
    /// ```
    /// # use datex_core::values::datex_type::CoreValueType;
    /// # use datex_core::values::value::Value;
    /// let value = Value::from(42);
    /// let casted_value = value.try_cast_to(CoreValueType::Text);
    /// assert!(casted_value.is_some());
    /// assert_eq!(casted_value.unwrap().get_type(), CoreValueType::Text);
    /// ```
    pub fn try_cast_to(&self, target_type: CoreValueType) -> Option<Value> {
        self.inner.cast_to(target_type.clone()).map(|inner| Value {
            actual_type: Box::new(inner.get_default_type()),
            inner, // Box::new(Type::new(
                   //     "core:fixme",
                   //     TypeDescriptor::Core(target_type),
                   // )),
        })
    }

    /// Casts the value to the target type, returning a Value.
    /// If the cast fails, it panics with an error message.
    /// This is useful for cases where you expect the cast to succeed and want to avoid handling the failure.
    /// # Arguments
    /// * `target_type` - The target type to cast the value to.
    /// # Returns
    /// * `Value` - The casted value.
    /// # Panics
    /// * If the cast fails, it panics with an error message.
    /// # Example
    /// ```
    /// # use datex_core::values::datex_type::CoreValueType;
    /// # use datex_core::values::value::Value;
    /// let value = Value::from(42);
    /// let casted_value = value.cast_to(CoreValueType::Text);
    /// assert_eq!(casted_value.get_type(), CoreValueType::Text);
    /// assert_eq!(casted_value, "42".into());
    /// ```
    pub fn cast_to(&self, target_type: CoreValueType) -> Value {
        self.try_cast_to(target_type.clone()).unwrap_or_else(|| {
            panic!("Failed to cast value to target type: {target_type:?}")
        })
    }

    /// Casts the value to the target type, returning a Value.
    /// If the cast fails, it returns a Value with type Null.
    /// This is similar to `cast_to`, but it returns a Value instead of an Option<Value>.
    /// # Arguments
    /// * `target_type` - The target type to cast the value to.
    /// # Returns
    /// * `Value` - The casted value, or a Value::null() if the cast fails.
    /// # Example
    /// ```
    /// # use datex_core::values::datex_type::CoreValueType;
    /// # use datex_core::values::value::Value;
    /// let value = Value::from(42);
    /// let casted_value = value.cast_or_null(CoreValueType::Text);
    /// assert_eq!(casted_value.get_type(), CoreValueType::Text);
    /// assert_eq!(casted_value.inner.cast_to_text().0, "42".to_string());
    /// ```
    pub fn cast_or_null(&self, target_type: CoreValueType) -> Value {
        self.try_cast_to(target_type).unwrap_or(Value::null())
    }

    // FIXME deprecate
    pub fn get_type(&self) -> CoreValueType {
        // self.actual_type.clone()
        todo!()
    }

    pub fn null() -> Self {
        CoreValue::Null.into()
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
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
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
        assert_structural_eq, datex_array,
        logger::init_logger_debug,
        values::core_values::{
            array::Array,
            endpoint::Endpoint,
            integer::{integer::Integer, typed_integer::TypedInteger},
        },
    };
    use log::{debug, info};
    use std::str::FromStr;

    #[test]
    fn endpoint() {
        init_logger_debug();
        let endpoint = Value::from(Endpoint::from_str("@test").unwrap());
        assert_eq!(endpoint.get_type(), CoreValueType::Endpoint);
        assert_eq!(endpoint.to_string(), "@test");

        let endpoint = Value::from("@test").cast_to(CoreValueType::Endpoint);
        assert_eq!(endpoint.get_type(), CoreValueType::Endpoint);
        assert_eq!(endpoint.to_string(), "@test");
    }

    #[test]
    fn new_addition_assignments() {
        let mut x = Value::from(42i8);
        let y = Value::from(27i8);

        x += y.clone();
        assert_eq!(x.get_type(), CoreValueType::I8);
        assert_eq!(x, Value::from(69i8));
    }

    #[test]
    fn new_additions() {
        let x = Value::from(42i8);
        let y = Value::from(27i8);

        let z = (x.clone() + y.clone()).unwrap();
        assert_eq!(z.get_type(), CoreValueType::I8);
        assert_eq!(z, Value::from(69i8));
    }

    #[test]
    fn array() {
        init_logger_debug();
        let mut a: Array = CoreValue::from(vec![
            Value::from("42"),
            Value::from(42),
            Value::from(true),
        ])
        .try_into()
        .unwrap();

        a.push(Value::from(42));
        a.push(4);

        assert_eq!(a.len(), 5);

        let b = Array::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.len(), 11);

        let c = datex_array![1, "test", 3, true, false];
        assert_eq!(c.len(), 5);
        assert_eq!(c[0], 1.into());
        assert_eq!(c[1], "test".into());
        assert_eq!(c[2], 3.into());
        debug!("Array: {c}");
    }

    #[test]
    fn boolean() {
        init_logger_debug();
        let a = Value::from(true);
        let b = Value::from(false);
        let c = Value::from(false);
        assert_eq!(a.get_type(), CoreValueType::Boolean);
        assert_eq!(b.get_type(), CoreValueType::Boolean);
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

        assert_eq!(a.get_type(), CoreValueType::I8);
        assert_eq!(b.get_type(), CoreValueType::I8);
        assert_eq!(c.get_type(), CoreValueType::I8);

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

        assert_eq!(a.get_type(), CoreValueType::F32);
        assert_eq!(b.get_type(), CoreValueType::F32);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b.get_type(), CoreValueType::F32);
        assert_eq!(a_plus_b, Value::from(69.1f32));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn cast_type() {
        init_logger_debug();
        let a = Value::from(42);
        let b = a.try_cast_to(CoreValueType::Text).unwrap();
        assert_eq!(b.get_type(), CoreValueType::Text);
    }

    #[test]
    fn null() {
        init_logger_debug();

        let null_value = Value::null();
        assert_eq!(null_value.get_type(), CoreValueType::Null);
        assert_eq!(null_value.to_string(), "null");

        let maybe_value: Option<i8> = None;
        let null_value = Value::from(maybe_value);
        assert_eq!(null_value.get_type(), CoreValueType::Null);
        assert_eq!(null_value.to_string(), "null");
    }

    #[test]
    fn addition() {
        init_logger_debug();
        let a = Value::from(42i8);
        let b = Value::from(27i8);

        assert_eq!(a.get_type(), CoreValueType::I8);
        assert_eq!(b.get_type(), CoreValueType::I8);

        let a_plus_b = (a.clone() + b.clone()).unwrap();

        assert_eq!(a_plus_b.get_type(), CoreValueType::I8);

        assert_eq!(a_plus_b, Value::from(69i8));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn string_concatenation() {
        init_logger_debug();
        let a = Value::from("Hello ");
        let b = Value::from(42i8);

        assert_eq!(a.get_type(), CoreValueType::Text);
        assert_eq!(b.get_type(), CoreValueType::I8);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        let b_plus_a = (b.clone() + a.clone()).unwrap();

        assert_eq!(a_plus_b.get_type(), CoreValueType::Text);
        assert_eq!(b_plus_a.get_type(), CoreValueType::Text);

        assert_eq!(a_plus_b, Value::from("Hello 42"));
        assert_eq!(b_plus_a, Value::from("42Hello "));

        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
        info!("{} + {} = {}", b.clone(), a.clone(), b_plus_a);
    }

    #[test]
    fn structural_equality() {
        let a = Value::from(42_i8);
        let b = Value::from(42_i32);

        assert_eq!(a.get_type(), CoreValueType::I8);
        assert_eq!(b.get_type(), CoreValueType::I32);

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
}

use crate::datex_values::core_value::CoreValue;
use crate::datex_values::reference::Reference;
use crate::datex_values::soft_eq::SoftEq;
use crate::datex_values::value_container::ValueError;
use log::error;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Not};

use super::datex_type::CoreValueType;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Value {
    pub inner: CoreValue,
    pub actual_type: CoreValueType, // custom type for the value that can not be changed
}
impl SoftEq for Value {
    fn soft_eq(&self, other: &Self) -> bool {
        self.inner.soft_eq(&other.inner)
    }
}

impl<T: Into<CoreValue>> From<T> for Value {
    fn from(inner: T) -> Self {
        let inner = inner.into();
        let actual_type = inner.get_default_type();
        Value { inner, actual_type }
    }
}

impl Value {
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
        self.is_of_type(CoreValueType::Bool)
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
    /// # use datex_core::datex_values::datex_type::CoreValueType;
    /// # use datex_core::datex_values::value::Value;
    /// let value = Value::from(42);
    /// let casted_value = value.try_cast_to(CoreValueType::Text);
    /// assert!(casted_value.is_some());
    /// assert_eq!(casted_value.unwrap().get_type(), CoreValueType::Text);
    /// ```
    pub fn try_cast_to(&self, target_type: CoreValueType) -> Option<Value> {
        self.inner.cast_to(target_type.clone()).map(|inner| Value {
            inner,
            actual_type: target_type,
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
    /// # use datex_core::datex_values::datex_type::CoreValueType;
    /// # use datex_core::datex_values::value::Value;
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
    /// # use datex_core::datex_values::datex_type::CoreValueType;
    /// # use datex_core::datex_values::value::Value;
    /// let value = Value::from(42);
    /// let casted_value = value.cast_or_null(CoreValueType::Text);
    /// assert_eq!(casted_value.get_type(), CoreValueType::Text);
    /// assert_eq!(casted_value.inner.cast_to_text().0, "42".to_string());
    /// ```
    pub fn cast_or_null(&self, target_type: CoreValueType) -> Value {
        self.try_cast_to(target_type).unwrap_or(Value::null())
    }

    pub fn get_type(&self) -> CoreValueType {
        self.actual_type.clone()
    }

    pub fn null() -> Self {
        CoreValue::Null.into()
    }
}

impl From<Reference> for Value {
    fn from(pointer: Reference) -> Self {
        pointer.value
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

impl Not for Value {
    type Output = Option<Value>;

    fn not(self) -> Self::Output {
        (!self.inner).map(Value::from)
    }
}

// TODO: crate a TryAddAssign trait etc.
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
            error!("Failed to add value: {:?}", res);
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

            // FIXME we should not use the type inference here
            None => Value::null(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::datex_values::core_values::integer::{Integer, TypedInteger};
    use crate::{
        assert_soft_eq, datex_array,
        datex_values::core_values::{array::Array, endpoint::Endpoint},
        logger::init_logger,
    };
    use log::{debug, info};
    use std::str::FromStr;

    #[test]
    fn test_endpoint() {
        init_logger();
        let endpoint = Value::from(Endpoint::from_str("@test").unwrap());
        assert_eq!(endpoint.get_type(), CoreValueType::Endpoint);
        assert_eq!(endpoint.to_string(), "@test");

        let endpoint = Value::from("@test").cast_to(CoreValueType::Endpoint);
        assert_eq!(endpoint.get_type(), CoreValueType::Endpoint);
        assert_eq!(endpoint.to_string(), "@test");

        let endpoint: Endpoint = CoreValue::from("@test").try_into().unwrap();
        debug!("Endpoint: {}", endpoint);
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
        init_logger();
        let mut a: Array = CoreValue::from(vec![
            Value::from("42"),
            Value::from(42),
            Value::from(true),
        ])
        .try_into()
        .unwrap();

        a.push(Value::from(42));
        a.push(4);

        assert_eq!(a.length(), 5);

        let b = Array::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.length(), 11);

        let c = datex_array![1, "test", 3, true, false];
        assert_eq!(c.length(), 5);
        assert_eq!(c[0], 1.into());
        assert_eq!(c[1], "test".into());
        assert_eq!(c[2], 3.into());
        debug!("Array: {}", c);
    }

    #[test]
    fn boolean() {
        init_logger();
        let a = Value::from(true);
        let b = Value::from(false);
        let c = Value::from(false);
        assert_eq!(a.get_type(), CoreValueType::Bool);
        assert_eq!(b.get_type(), CoreValueType::Bool);
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
        init_logger();
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
        init_logger();
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
    fn test_cast_type() {
        init_logger();
        let a = Value::from(42);
        let b = a.try_cast_to(CoreValueType::Text).unwrap();
        assert_eq!(b.get_type(), CoreValueType::Text);
    }

    #[test]
    fn test_infer_type() {
        init_logger();
        let a = CoreValue::from(42i32);
        let b = CoreValue::from(11i32);
        let c = CoreValue::from("11");

        assert_eq!(a.get_default_type(), CoreValueType::I32);
        assert_eq!(b.get_default_type(), CoreValueType::I32);
        assert_eq!(c.get_default_type(), CoreValueType::Text);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b.clone().get_default_type(), CoreValueType::I32);
        assert_eq!(a_plus_b.clone(), CoreValue::from(53));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b.clone());
    }

    #[test]
    fn test_null() {
        init_logger();

        let null_value = Value::null();
        assert_eq!(null_value.get_type(), CoreValueType::Null);
        assert_eq!(null_value.to_string(), "null");

        let maybe_value: Option<i8> = None;
        let null_value = Value::from(maybe_value);
        assert_eq!(null_value.get_type(), CoreValueType::Null);
        assert_eq!(null_value.to_string(), "null");
    }

    #[test]
    fn test_addition() {
        init_logger();
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
    fn test_string_concatenation() {
        init_logger();
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
    fn test_soft_equals() {
        let a = Value::from(42_i8);
        let b = Value::from(42_i32);

        assert_eq!(a.get_type(), CoreValueType::I8);
        assert_eq!(b.get_type(), CoreValueType::I32);

        assert_soft_eq!(a, b);

        assert_eq!(
            Value::from(Integer(TypedInteger::I8(42))),
            Value::from(Integer(TypedInteger::U32(42))),
        );

        assert_soft_eq!(
            Value::from(Integer(TypedInteger::I8(42))),
            Value::from(Integer(TypedInteger::U32(42))),
        );

        assert_soft_eq!(Value::from(42_i8), Value::from(Integer::from(42_i8)));
    }
}

// #[test]
// fn test_text() {
//     init_logger();
//     let a = CoreValue::from("Hello");
//     assert_eq!(a, "Hello");
//     assert_eq!(a.get_type(), CoreValueType::Text);
//     assert_eq!(a.length(), 5);
//     assert_eq!(a.to_string(), "\"Hello\"");
//     assert_eq!(a.as_str(), "Hello");
//     assert_eq!(a.to_uppercase(), "HELLO".into());
//     assert_eq!(a.to_lowercase(), "hello".into());
//
//     let b = &mut TypedValue::from("World");
//     b.reverse();
//     assert_eq!(b.length(), 5);
//     assert_eq!(b.as_str(), "dlroW");
// }

// #[test]
// /// A TypedDatexValue<T> should allow custom TypedDatexValue<X> to be added to it.
// /// This won't change the type of the TypedDatexValue<T> but will allow the value to be modified.
// /// A untyped DatexValue can be assigned to TypedDatexValue<T> but this might throw an error if the type is not compatible.
// fn test_test_assign1() {
//     init_logger();
//     let mut a: TypedValue<Text> = TypedValue::from("Hello");
//     a += " World"; // see (#2)
//     a += TypedValue::from("4"); // Is typesafe
//     a += 2;
//     a += TypedValue::from(42); // Is typesafe see (#1)
//                                // We won't allow this: `a += TypedDatexValue::from(true);`
//     a += Value::from("!"); // Might throw if the assignment would be incompatible.
//     assert_eq!(a.length(), 16);
//     assert_eq!(a.as_str(), "Hello World4242!");
// }
//
// #[test]
// fn test_test_assign2() {
//     init_logger();
//     let mut a = TypedValue::from("Hello");
//     a += " World";
//     a += Value::from("!");
//
//     assert_eq!(a.length(), 12);
//     assert_eq!(a.as_str(), "Hello World!");
//
//     a += 42;
//
//     assert_eq!(a.length(), 14);
//     assert_eq!(a.as_str(), "Hello World!42");
//
//     let mut b = Value::from("Hello");
//     b += " World ";
//     b += TypedValue::from(42);
//     b += Value::from("!");
//
//     let b = b.cast_to_typed::<Text>();
//
//     info!("{}", b);
//     assert_eq!(b.length(), 15);
//     assert_eq!(b.as_str(), "Hello World 42!");
// }
//
// #[test]
// fn test_typed_addition() {
//     init_logger();
//     let a = TypedValue::from(42);
//     let b = TypedValue::from(27);
//
//     assert_eq!(a, 42);
//     assert_eq!(b, 27);
//
//     assert_eq!(a.get_type(), CoreValueType::I8);
//     assert_eq!(b.get_type(), CoreValueType::I8);
//
//     let a_plus_b = a.clone() + b.clone();
//
//     assert_eq!(a_plus_b.get_type(), CoreValueType::I8);
//
//     assert_eq!(a_plus_b, TypedValue::from(69));
//     info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
// }

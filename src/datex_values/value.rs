use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Not};
use log::error;
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::pointer::Pointer;
use crate::datex_values::value_container::{ValueError};

use super::core_values::null::Null;
use super::datex_type::CoreValueType;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Value {
    pub inner: CoreValue,
    pub actual_type: CoreValueType, // custom type for the value that can not be changed
}

impl<T: Into<CoreValue>> From<T> for Value {
    fn from(inner: T) -> Self {
        let inner = inner.into();
        let actual_type = inner.get_default_type();
        Value {
            inner,
            actual_type,
        }
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

    pub fn cast_to(&self, target_type: CoreValueType) -> Option<Value> {
        self.inner.cast_to(target_type.clone()).map(|inner| Value {
            inner,
            actual_type: target_type,
        })
    }

    pub fn get_type(&self) -> CoreValueType {
        self.actual_type.clone()
    }

    pub fn null() -> Self {
        CoreValue::Null(Null).into()
    }
}


impl From<Pointer> for Value {
    fn from(pointer: Pointer) -> Self {
        pointer.value
    }
}

// impl PartialEq for Value {
//     fn eq(&self, other: &Self) -> bool {
//         if self.actual_type == other.actual_type {
//             self.inner == other.inner
//         } else {
//             false
//         }
//     }
// }

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
        (!self.inner).map(|inner| Value::from(inner))
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
    use std::str::FromStr;
    use super::*;
    use crate::{
        datex_array,
        datex_values::core_values::{array::Array, endpoint::Endpoint},
        logger::init_logger,
    };
    use log::{debug, info};

    #[test]
    fn test_endpoint() {
        init_logger();
        let endpoint = Value::from(Endpoint::from_str("@test").unwrap());
        debug!("Endpoint: {}", endpoint);
        assert_eq!(endpoint.get_type(), CoreValueType::Endpoint);
        assert_eq!(endpoint.to_string(), "@test");
    }

    #[test]
    fn new_addition_assignments() {
        let mut x = Value::from(42);
        let y = Value::from(27);

        x += y.clone();
        assert_eq!(x.get_type(), CoreValueType::I8);
        assert_eq!(x, Value::from(69));
    }

    #[test]
    fn new_additions() {
        let x = Value::from(42);
        let y = Value::from(27);

        let z = (x.clone() + y.clone()).unwrap();
        assert_eq!(z.get_type(), CoreValueType::I8);
        assert_eq!(z, Value::from(69));
    }

    #[test]
    fn array() {
        init_logger();
        let mut a = Value::from(vec![
            Value::from("42"),
            Value::from(42),
            Value::from(true),
        ]);

        if let CoreValue::Array(a) = &mut a.inner {
            a.push(Value::from(42));
            a.push(4);

            assert_eq!(a.length(), 5);
            debug!("Array: {}", a);
        } else {
            panic!("Expected Array type");
        }

        let b = Array::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.length(), 11);

        let c = datex_array![1, "test", 3, true, false];
        assert_eq!(c.length(), 5);
        assert_eq!(c[0], 1.into());
        assert_eq!(c[1], "test".into());
        assert_eq!(c[2], 3.into());

        //c.insert(0, 1.into());
        debug!("Array: {}", c);
    }

    // TODO: think about a serialization/deserialization strategy in combination with the compiler
    // #[test]
    // fn serialize() {
    //     init_logger();
    //     test_serialize_and_deserialize(Value::from(42));
    //     test_serialize_and_deserialize(Value::from("Hello World!"));
    //     test_serialize_and_deserialize(Value::from(true));
    //     test_serialize_and_deserialize(Value::from(false));
    //     test_serialize_and_deserialize(Value::null());
    //     test_serialize_and_deserialize(Value::from(0));
    //     test_serialize_and_deserialize(Value::from(1));
    // }

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
    fn test_cast_type() {
        init_logger();
        let a = Value::from(42);
        let b = a.cast_to(CoreValueType::Text).unwrap();
        assert_eq!(b.get_type(), CoreValueType::Text);
    }

    #[test]
    fn test_infer_type() {
        init_logger();
        let a = CoreValue::from(42);
        let b = CoreValue::from(11);
        let c = CoreValue::from("11");

        assert_eq!(a.get_default_type(), CoreValueType::I8);
        assert_eq!(b.get_default_type(), CoreValueType::I8);
        assert_eq!(c.get_default_type(), CoreValueType::Text);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b.clone().get_default_type(), CoreValueType::I8);
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

    #[test]
    fn test_addition() {
        init_logger();
        let a = Value::from(42);
        let b = Value::from(27);

        assert_eq!(a.get_type(), CoreValueType::I8);
        assert_eq!(b.get_type(), CoreValueType::I8);

        let a_plus_b = (a.clone() + b.clone()).unwrap();

        assert_eq!(a_plus_b.get_type(), CoreValueType::I8);

        assert_eq!(a_plus_b, Value::from(69));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn test_string_concatenation() {
        init_logger();
        let a = Value::from("Hello ");
        let b = Value::from(42i8);

        assert_eq!(a.get_type(), CoreValueType::Text);
        assert_eq!(b.get_type(), CoreValueType::I8);

        let a_plus_b = a.clone() + b.clone();
        let b_plus_a = b.clone() + a.clone();

        info!("a: {}", a);
        info!("b: {}", b);
        info!("a + b: {:?}", b_plus_a);

        return;

        let a_plus_b = a_plus_b.unwrap();
        let b_plus_a = b_plus_a.unwrap();

        assert_eq!(a_plus_b.get_type(), CoreValueType::Text);
        assert_eq!(b_plus_a.get_type(), CoreValueType::Text);

        assert_eq!(a_plus_b, Value::from("Hello 42"));
        assert_eq!(b_plus_a, Value::from("42Hello "));

        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
        info!("{} + {} = {}", b.clone(), a.clone(), b_plus_a);
    }

    // fn serialize_datex_value(value: &Value) -> String {
    //     let res = serde_json::to_string(value).unwrap();
    //     info!("Serialized DatexValue: {}", res);
    //     res
    // }
    // fn deserialize_datex_value(json: &str) -> Value {
    //     let res = serde_json::from_str(json).unwrap();
    //     info!("Deserialized DatexValue: {}", res);
    //     res
    // }
    // fn test_serialize_and_deserialize(value: Value) {
    //     let json = serialize_datex_value(&value);
    //     let deserialized = deserialize_datex_value(&json);
    //     assert_eq!(value, deserialized);
    // }
}

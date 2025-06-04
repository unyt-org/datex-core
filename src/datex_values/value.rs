use std::ops::{Add, AddAssign, Not};
use std::vec;

use serde::{de, Deserialize, Deserializer, Serialize};
use crate::datex_values::pointer::Pointer;
use crate::datex_values::value_container::{ValueContainer, ValueError};
// Array<?>
// $0 = Int
// $1 = Bool
// [$0, $1, Text] // [3]
// get(0)

// Array<String>
// Box<len, data>

use super::core_value::{try_cast_to_value_dyn, CoreValue};
use super::core_values::array::DatexArray;
use super::core_values::bool::Bool;
use super::core_values::endpoint::Endpoint;
use super::core_values::int::I8;
use super::core_values::null::Null;
use super::core_values::text::Text;
use super::datex_type::Type;
use super::typed_value::TypedValue;

#[derive(Clone, Debug, PartialEq)]
pub enum DatexValueInner {
    Bool(Bool),
    I8(I8),
    Text(Text),
    Null(Null),
    Endpoint(Endpoint),
    Array(DatexArray),
}


impl DatexValueInner {
    pub fn to_dyn(&self) -> &dyn CoreValue {
        match &self {
            DatexValueInner::Bool(v) => v,
            DatexValueInner::I8(v) => v,
            DatexValueInner::Text(v) => v,
            DatexValueInner::Null(v) => v,
            DatexValueInner::Endpoint(v) => v,
            DatexValueInner::Array(v) => v,
        }
    }
    pub fn to_dyn_mut(&mut self) -> &mut dyn CoreValue {
        match self {
            DatexValueInner::Bool(v) => v,
            DatexValueInner::I8(v) => v,
            DatexValueInner::Text(v) => v,
            DatexValueInner::Null(v) => v,
            DatexValueInner::Endpoint(v) => v,
            DatexValueInner::Array(v) => v,
        }
    }
}

impl<V: CoreValue> From<&V> for DatexValueInner {
    fn from(value: &V) -> Self {
        // FIMXE deprecate as_any
        match value.get_type() {
            Type::Bool => DatexValueInner::Bool(
                value.as_any().downcast_ref::<Bool>().unwrap().clone(),
            ),
            Type::I8 => DatexValueInner::I8(
                value.as_any().downcast_ref::<I8>().unwrap().clone(),
            ),
            Type::Text => DatexValueInner::Text(
                value.as_any().downcast_ref::<Text>().unwrap().clone(),
            ),
            Type::Null => DatexValueInner::Null(
                value.as_any().downcast_ref::<Null>().unwrap().clone(),
            ),
            Type::Array => DatexValueInner::Array(
                value.as_any().downcast_ref::<DatexArray>().unwrap().clone(),
            ),
            Type::Endpoint => DatexValueInner::Endpoint(
                value.as_any().downcast_ref::<Endpoint>().unwrap().clone(),
            ),
        }
    }
}

#[derive(Clone)]
pub struct Value {
    pub inner: DatexValueInner,
    pub actual_type: Type, // custom type for the value that can not be changed
}

impl<T: CoreValue + 'static> From<TypedValue<T>> for Value {
    fn from(typed: TypedValue<T>) -> Self {
        Value {
            inner: DatexValueInner::from(typed.inner()).clone(),
            actual_type: typed.get_type(),
        }
    }
}

impl Value {
    pub fn is_of_type(&self, target: Type) -> bool {
        self.get_type() == target
    }
    pub fn is_null(&self) -> bool {
        self.is_of_type(Type::Null)
    }
    pub fn is_text(&self) -> bool {
        self.is_of_type(Type::Text)
    }
    pub fn is_i8(&self) -> bool {
        self.is_of_type(Type::I8)
    }
    pub fn is_bool(&self) -> bool {
        self.is_of_type(Type::Bool)
    }
}

impl Value {
    pub fn to_dyn(&self) -> &dyn CoreValue {
        self.inner.to_dyn()
    }

    pub fn to_dyn_mut(&mut self) -> &mut dyn CoreValue {
        self.inner.to_dyn_mut()
    }

    pub fn get_casted_inners<'a>(
        lhs: Value,
        rhs: &Value,
    ) -> Option<(DatexValueInner, DatexValueInner)> {
        let rhs = rhs.to_dyn();
        let rhs = rhs.cast_to(lhs.actual_type.clone())?;
        Some((lhs.inner, rhs.inner))
    }
    pub fn get_casted_inners_mut<'a>(
        lhs: &'a mut Value,
        rhs: &Value,
    ) -> Option<(&'a mut DatexValueInner, DatexValueInner)> {
        let rhs = rhs.to_dyn();
        let rhs = rhs.cast_to(lhs.actual_type.clone())?;
        Some((&mut lhs.inner, rhs.inner))
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.to_dyn().as_any().downcast_ref::<T>()
    }
    pub fn boxed<V: CoreValue + 'static>(v: V) -> Self {
        Value {
            inner: DatexValueInner::from(&v),
            actual_type: V::static_type(),
        }
    }

    pub fn cast_to(&self, target: Type) -> Option<Value> {
        self.to_dyn().cast_to(target)
    }
    pub fn try_cast_to_typed<T: CoreValue + Clone + 'static>(
        &self,
    ) -> Result<TypedValue<T>, ()> {
        let casted = self.cast_to(T::static_type()).ok_or(())?;
        let casted = casted
            .to_dyn()
            .as_any()
            .downcast_ref::<T>()
            .map(|v| TypedValue(v.clone()));
        casted.ok_or(())
    }

    pub fn try_cast_to_value<T: CoreValue + Clone + 'static>(
        &self,
    ) -> Result<T, ()> {
        try_cast_to_value_dyn(self.to_dyn())
    }

    pub fn cast_to_typed<T: CoreValue + Clone + 'static>(
        &self,
    ) -> TypedValue<T> {
        self.try_cast_to_typed::<T>().unwrap_or_else(|_| {
            panic!("Failed to cast to type: {:?}", T::static_type())
        })
    }

    pub fn get_type(&self) -> Type {
        self.actual_type.clone()
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<ValueContainer>,
{
    fn from(vec: Vec<T>) -> Self {
        let items = vec.into_iter().map(Into::into).collect();
        Value::from(DatexArray(items))
    }
}
impl From<DatexArray> for Value {
    fn from(arr: DatexArray) -> Self {
        Value::boxed(arr)
    }
}

impl From<Pointer> for Value {
    fn from(pointer: Pointer) -> Self {
        pointer.value
    }
}

impl Value {
    pub fn null() -> Self {
        Value::boxed(Null)
    }
}
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.actual_type == other.actual_type {
            self.inner == other.inner
        } else {
            false
        }
    }
}

impl Add for Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: Value) -> Self::Output {
        // TODO sync with typed_datex_values
        match (self.inner, rhs.inner) {
            (DatexValueInner::Text(text), other)
            | (other, DatexValueInner::Text(text)) => {
                let other =
                    try_cast_to_value_dyn::<Text>(other.to_dyn())
                        .map_err(|_| ValueError::TypeConversionError)?;
                let text = text.add(other);
                Ok(text.as_datex_value())
            }
            (DatexValueInner::I8(lhs), DatexValueInner::I8(rhs)) => {
                Ok(lhs.add(rhs).as_datex_value())
            }
            _ => Err(ValueError::InvalidOperation),
        }
    }
}

impl Add for &Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: &Value) -> Self::Output {
        // TODO sync with typed_datex_values
        match (&self.inner, &rhs.inner) {
            (DatexValueInner::Text(text), other) => {
                let other =
                    try_cast_to_value_dyn::<Text>(other.to_dyn())
                        .map_err(|_| ValueError::TypeConversionError)?;
                let text = text + other;
                Ok(text.as_datex_value())
            }
            (other, DatexValueInner::Text(text)) => {
                let other =
                    try_cast_to_value_dyn::<Text>(other.to_dyn())
                        .map_err(|_| ValueError::TypeConversionError)?;
                let text = other + text;
                Ok(text.as_datex_value())
            }
            (DatexValueInner::I8(lhs), DatexValueInner::I8(rhs)) => {
                Ok(lhs.add(rhs).as_datex_value())
            }
            _ => Err(ValueError::InvalidOperation),
        }
    }
}

impl Not for Value {
    type Output = Option<Value>;

    fn not(self) -> Self::Output {
        if let Ok(typed) = self.try_cast_to_typed::<Bool>() {
            Some(Value::from(!typed.inner().0))
        } else {
            None
        }
    }
}

impl<T> AddAssign<T> for Value
where
    Value: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        let rhs: Value = rhs.into();
        // TODO sync with typed_datex_values
        match (&mut self.inner, rhs.inner) {
            (DatexValueInner::Text(text), other) => {
                let other = try_cast_to_value_dyn::<Text>(other.to_dyn())
                    .expect("Failed to cast");
                text.add_assign(other);
            }
            (DatexValueInner::I8(lhs), DatexValueInner::I8(rhs)) => {
                lhs.add_assign(rhs);
            }
            _ => panic!("Unsupported addition"),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_dyn())
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_dyn().fmt(f)
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
        datex_values::core_values::{array::DatexArray, endpoint::Endpoint},
        logger::init_logger,
    };
    use log::{debug, info};

    #[test]
    fn test_endpoint() {
        init_logger();
        let endpoint = Value::from(Endpoint::from_str("@test").unwrap());
        debug!("Endpoint: {}", endpoint);
        assert_eq!(endpoint.get_type(), Type::Endpoint);
        assert_eq!(endpoint.to_string(), "@test");

        let a = TypedValue::from(Endpoint::from_str("@test").unwrap());
        assert_eq!(a.get_type(), Type::Endpoint);
        assert_eq!(a.to_string(), "@test");
    }

    #[test]
    fn new_addition_assignments() {
        let mut x = Value::from(42);
        let y = Value::from(27);

        x += y.clone();
        assert_eq!(x.get_type(), Type::I8);
        assert_eq!(x, Value::from(69));
    }

    #[test]
    fn new_additions() {
        let x = Value::from(42);
        let y = Value::from(27);

        let z = (x.clone() + y.clone()).unwrap();
        assert_eq!(z.get_type(), Type::I8);
        assert_eq!(z, Value::from(69));
    }

    #[test]
    fn array() {
        init_logger();
        let a = Value::from(vec![
            Value::from("42"),
            Value::from(42),
            Value::from(true),
        ]);

        let mut a = a.cast_to_typed::<DatexArray>();
        a.push(Value::from(42));
        a.push(4);
        a += 42;
        a += DatexArray::from(vec!["inner", "array"]);
        let a: DatexArray = a.into_inner();

        assert_eq!(a.length(), 7);
        debug!("Array: {}", a);

        let b = DatexArray::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.length(), 11);
        assert_eq!(b.get_type(), Type::Array);

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
    fn typed_boolean() {
        init_logger();
        let a = TypedValue::from(true);
        let b = TypedValue::from(false);

        assert_eq!(a.get_type(), Type::Bool);
        assert_eq!(b.get_type(), Type::Bool);
        assert_ne!(a, b);
        assert_eq!(b, false);
        assert_eq!(!a, b);

        let mut a = TypedValue::from(true);
        let b = TypedValue::from(false);
        a.toggle();
        assert_eq!(a, b);
    }

    #[test]
    fn boolean() {
        init_logger();
        let a = Value::from(true);
        let b = Value::from(false);
        let c = Value::from(false);
        assert_eq!(a.get_type(), Type::Bool);
        assert_eq!(b.get_type(), Type::Bool);
        assert_ne!(a, b);
        assert_eq!(b, c);

        let d = (!b.clone()).unwrap();
        assert_eq!(a, d);

        // We can't add two booleans together, so this should return None
        let a_plus_b = a.clone() + b.clone();
        assert!(a_plus_b.is_err());
    }

    #[test]
    fn type_casting_into() {
        init_logger();
        let a: TypedValue<Text> = Value::from("42").try_into().unwrap();
        assert_eq!(a.get_type(), Type::Text);

        let a: TypedValue<Text> = Value::from(42).try_into().unwrap();
        assert_eq!(a.get_type(), Type::Text);

        // This should fail because we are trying to cast a null value into a TypedDatexValue<Text>
        let a: Result<TypedValue<Text>, _> = Value::null().try_into();
        assert!(a.is_err());
    }

    #[test]
    fn test_cast_type() {
        init_logger();
        let a = Value::from(42);
        let b = a.cast_to(Type::Text).unwrap();
        assert_eq!(b.get_type(), Type::Text);

        let c = a.cast_to_typed::<I8>();
        assert_eq!(c.into_erased(), Value::from(42));

        let d = a.cast_to_typed::<Text>();
        assert_eq!(d.get_type(), Type::Text);
        assert_eq!(d.as_str(), "42");
    }

    #[test]
    fn test_infer_type() {
        init_logger();
        let a = TypedValue::from(42);
        let b = TypedValue::from(11);
        let c = TypedValue::from("11");
        assert_eq!(c.length(), 2);

        assert_eq!(a.get_type(), Type::I8);
        assert_eq!(b.get_type(), Type::I8);

        let a_plus_b = a.clone() + b.clone();
        assert_eq!(a_plus_b.clone().get_type(), Type::I8);
        assert_eq!(a_plus_b.clone().into_erased(), Value::from(53));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b.clone());
    }

    #[test]
    fn test_null() {
        init_logger();

        let null_value = Value::null();
        assert_eq!(null_value.get_type(), Type::Null);
        assert_eq!(null_value.to_string(), "null");

        let maybe_value: Option<i8> = None;
        let null_value = Value::from(maybe_value);
        assert_eq!(null_value.get_type(), Type::Null);
        assert_eq!(null_value.to_string(), "null");
    }

    #[test]
    fn test_text() {
        init_logger();
        let a = TypedValue::from("Hello");
        assert_eq!(a, "Hello");
        assert_eq!(a.get_type(), Type::Text);
        assert_eq!(a.length(), 5);
        assert_eq!(a.to_string(), "\"Hello\"");
        assert_eq!(a.as_str(), "Hello");
        assert_eq!(a.to_uppercase(), "HELLO".into());
        assert_eq!(a.to_lowercase(), "hello".into());

        let b = &mut TypedValue::from("World");
        b.reverse();
        assert_eq!(b.length(), 5);
        assert_eq!(b.as_str(), "dlroW");
    }

    #[test]
    /// A TypedDatexValue<T> should allow custom TypedDatexValue<X> to be added to it.
    /// This won't change the type of the TypedDatexValue<T> but will allow the value to be modified.
    /// A untyped DatexValue can be assigned to TypedDatexValue<T> but this might throw an error if the type is not compatible.
    fn test_test_assign1() {
        init_logger();
        let mut a: TypedValue<Text> = TypedValue::from("Hello");
        a += " World"; // see (#2)
        a += TypedValue::from("4"); // Is typesafe
        a += 2;
        a += TypedValue::from(42); // Is typesafe see (#1)
                                   // We won't allow this: `a += TypedDatexValue::from(true);`
        a += Value::from("!"); // Might throw if the assignment would be incompatible.
        assert_eq!(a.length(), 16);
        assert_eq!(a.as_str(), "Hello World4242!");
    }

    #[test]
    fn test_test_assign2() {
        init_logger();
        let mut a = TypedValue::from("Hello");
        a += " World";
        a += Value::from("!");

        assert_eq!(a.length(), 12);
        assert_eq!(a.as_str(), "Hello World!");

        a += 42;

        assert_eq!(a.length(), 14);
        assert_eq!(a.as_str(), "Hello World!42");

        let mut b = Value::from("Hello");
        b += " World ";
        b += TypedValue::from(42);
        b += Value::from("!");

        let b = b.cast_to_typed::<Text>();

        info!("{}", b);
        assert_eq!(b.length(), 15);
        assert_eq!(b.as_str(), "Hello World 42!");
    }

    #[test]
    fn test_typed_addition() {
        init_logger();
        let a = TypedValue::from(42);
        let b = TypedValue::from(27);

        assert_eq!(a, 42);
        assert_eq!(b, 27);

        assert_eq!(a.get_type(), Type::I8);
        assert_eq!(b.get_type(), Type::I8);

        let a_plus_b = a.clone() + b.clone();

        assert_eq!(a_plus_b.get_type(), Type::I8);

        assert_eq!(a_plus_b, TypedValue::from(69));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn test_addition() {
        init_logger();
        let a = Value::from(42);
        let b = Value::from(27);

        assert_eq!(a.get_type(), Type::I8);
        assert_eq!(b.get_type(), Type::I8);

        let a_plus_b = (a.clone() + b.clone()).unwrap();

        assert_eq!(a_plus_b.get_type(), Type::I8);

        assert_eq!(a_plus_b, Value::from(69));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn test_string_concatenation() {
        init_logger();
        let a = Value::from("Hello ");
        let b = Value::from(42i8);

        assert_eq!(a.get_type(), Type::Text);
        assert_eq!(b.get_type(), Type::I8);

        let a_plus_b = a.clone() + b.clone();
        let b_plus_a = b.clone() + a.clone();

        info!("a: {}", a);
        info!("b: {}", b);
        info!("a + b: {:?}", b_plus_a);

        return;

        let a_plus_b = a_plus_b.unwrap();
        let b_plus_a = b_plus_a.unwrap();

        assert_eq!(a_plus_b.get_type(), Type::Text);
        assert_eq!(b_plus_a.get_type(), Type::Text);

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

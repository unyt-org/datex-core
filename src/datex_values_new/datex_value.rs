use std::fmt::Display;
use std::ops::{Add, AddAssign, Not};
use std::vec;

use serde::{de, Deserialize, Deserializer, Serialize};

use super::array::DatexArray;
use super::bool::Bool;
use super::datex_type::DatexType;
use super::int::I8;
use super::null::Null;
use super::text::Text;
use super::typed_datex_value::TypedDatexValue;
use super::value::{try_cast_to_value_dyn, Value};

#[derive(Clone, Debug)]
pub enum DatexValueInner {
    Bool(Bool),
    I8(I8),
    Text(Text),
    Null(Null),
    Array(DatexArray),
}

impl DatexValueInner {
    pub fn to_dyn(&self) -> &dyn Value {
        match &self {
            DatexValueInner::Bool(v) => v,
            DatexValueInner::I8(v) => v,
            DatexValueInner::Text(v) => v,
            DatexValueInner::Null(v) => v,
            DatexValueInner::Array(v) => v,
        }
    }
    pub fn to_dyn_mut(&mut self) -> &mut dyn Value {
        match self {
            DatexValueInner::Bool(v) => v,
            DatexValueInner::I8(v) => v,
            DatexValueInner::Text(v) => v,
            DatexValueInner::Null(v) => v,
            DatexValueInner::Array(v) => v,
        }
    }
}

impl<V: Value> From<&V> for DatexValueInner {
    fn from(value: &V) -> Self {
        match value.get_type() {
            DatexType::Bool => DatexValueInner::Bool(
                value.as_any().downcast_ref::<Bool>().unwrap().clone(),
            ),
            DatexType::I8 => DatexValueInner::I8(
                value.as_any().downcast_ref::<I8>().unwrap().clone(),
            ),
            DatexType::Text => DatexValueInner::Text(
                value.as_any().downcast_ref::<Text>().unwrap().clone(),
            ),
            DatexType::Null => DatexValueInner::Null(
                value.as_any().downcast_ref::<Null>().unwrap().clone(),
            ),
            DatexType::Array => DatexValueInner::Array(
                value.as_any().downcast_ref::<DatexArray>().unwrap().clone(),
            ),
        }
    }
}

#[derive(Clone)]
pub struct DatexValuePointer {
    pub value: DatexValue,
    pub allowed_type: DatexType, // custom type for the pointer that the Datex value can get
}

#[derive(Clone)]
pub enum DatexValueContainer {
    Value(DatexValue),
    Pointer(DatexValuePointer),
}

#[derive(Clone)]
pub struct DatexValue {
    pub inner: DatexValueInner, //Arc<dyn Value>,
    pub actual_type: DatexType, // custom type for the value that can not be changed
}

impl<T: Value + 'static> From<TypedDatexValue<T>> for DatexValue {
    fn from(typed: TypedDatexValue<T>) -> Self {
        DatexValue {
            inner: DatexValueInner::from(typed.inner()).clone(),
            actual_type: typed.get_type(),
        }
    }
}

impl DatexValue {
    pub fn is_of_type(&self, target: DatexType) -> bool {
        self.get_type() == target
    }
    pub fn is_null(&self) -> bool {
        self.is_of_type(DatexType::Null)
    }
    pub fn is_text(&self) -> bool {
        self.is_of_type(DatexType::Text)
    }
    pub fn is_i8(&self) -> bool {
        self.is_of_type(DatexType::I8)
    }
    pub fn is_bool(&self) -> bool {
        self.is_of_type(DatexType::Bool)
    }
}

impl DatexValue {
    pub fn to_dyn(&self) -> &dyn Value {
        self.inner.to_dyn()
    }

    pub fn to_dyn_mut(&mut self) -> &mut dyn Value {
        self.inner.to_dyn_mut()
    }

    pub fn get_casted_inners<'a>(
        lhs: DatexValue,
        rhs: &DatexValue,
    ) -> Option<(DatexValueInner, DatexValueInner)> {
        let rhs = rhs.to_dyn();
        let rhs = rhs.cast_to(lhs.actual_type.clone())?;
        Some((lhs.inner, rhs.inner))
    }
    pub fn get_casted_inners_mut<'a>(
        lhs: &'a mut DatexValue,
        rhs: &DatexValue,
    ) -> Option<(&'a mut DatexValueInner, DatexValueInner)> {
        let rhs = rhs.to_dyn();
        let rhs = rhs.cast_to(lhs.actual_type.clone())?;
        Some((&mut lhs.inner, rhs.inner))
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.to_dyn().as_any().downcast_ref::<T>()
    }
    pub fn boxed<V: Value + 'static>(v: V) -> Self {
        DatexValue {
            inner: DatexValueInner::from(&v),
            actual_type: V::static_type(),
        }
    }

    pub fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        self.to_dyn().cast_to(target)
    }
    pub fn try_cast_to_typed<T: Value + Clone + 'static>(
        &self,
    ) -> Result<TypedDatexValue<T>, ()> {
        let casted = self.cast_to(T::static_type()).ok_or(())?;
        let casted = casted
            .to_dyn()
            .as_any()
            .downcast_ref::<T>()
            .map(|v| TypedDatexValue(v.clone()));
        casted.ok_or(())
    }

    pub fn try_cast_to_value<T: Value + Clone + 'static>(
        &self,
    ) -> Result<T, ()> {
        try_cast_to_value_dyn(self.to_dyn())
    }

    pub fn cast_to_typed<T: Value + Clone + 'static>(
        &self,
    ) -> TypedDatexValue<T> {
        self.try_cast_to_typed::<T>().unwrap_or_else(|_| {
            panic!("Failed to cast to type: {:?}", T::static_type())
        })
    }

    pub fn get_type(&self) -> DatexType {
        self.actual_type.clone()
    }
}

#[derive(Serialize, Deserialize)]
#[serde()]
pub struct SerializableDatexValue {
    #[serde(rename = "type")]
    _type: DatexType,
    value: Vec<u8>,
}
impl SerializableDatexValue {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.push(self._type.clone() as u8);
        bytes.extend_from_slice(&self.value);
        bytes
    }
}
impl From<&DatexValue> for SerializableDatexValue {
    fn from(value: &DatexValue) -> Self {
        SerializableDatexValue {
            _type: value.get_type(),
            value: value.to_dyn().to_bytes(),
        }
    }
}
impl<T> From<Vec<T>> for DatexValue
where
    T: Into<DatexValue>,
{
    fn from(vec: Vec<T>) -> Self {
        let items = vec.into_iter().map(Into::into).collect();
        DatexValue::from(DatexArray(items))
    }
}
impl From<DatexArray> for DatexValue {
    fn from(arr: DatexArray) -> Self {
        DatexValue::boxed(arr)
    }
}
impl Serialize for DatexValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let repr: SerializableDatexValue = self.into();
        repr.serialize(serializer)
    }
}
impl TryFrom<SerializableDatexValue> for DatexValue {
    type Error = String;

    fn try_from(dxvalue: SerializableDatexValue) -> Result<Self, Self::Error> {
        match dxvalue._type {
            DatexType::Text => {
                let text = Text::from_bytes(&dxvalue.value);
                Ok(DatexValue::boxed(text))
            }
            DatexType::I8 => {
                let i8 = I8::from_bytes(&dxvalue.value);
                Ok(DatexValue::boxed(i8))
            }
            DatexType::Bool => {
                let bool = Bool::from_bytes(&dxvalue.value);
                Ok(DatexValue::boxed(bool))
            }
            DatexType::Null => Ok(DatexValue::null()),
            _ => Err(format!("Unsupported type: {:?}", dxvalue.value)),
        }
    }
}

impl DatexValue {
    pub fn null() -> Self {
        DatexValue::boxed(Null)
    }
}
impl PartialEq for DatexValue {
    fn eq(&self, other: &Self) -> bool {
        if self.actual_type == other.actual_type {
            match (&self.inner, &other.inner) {
                (DatexValueInner::Bool(a), DatexValueInner::Bool(b)) => a == b,
                (DatexValueInner::I8(a), DatexValueInner::I8(b)) => a == b,
                (DatexValueInner::Text(a), DatexValueInner::Text(b)) => a == b,
                (DatexValueInner::Null(_), DatexValueInner::Null(_)) => true,
                (DatexValueInner::Array(a), DatexValueInner::Array(b)) => false, // TODO
                _ => false,
            }
        } else {
            false
        }
    }
}

impl Add for DatexValue {
    type Output = Option<DatexValue>;
    fn add(self, rhs: DatexValue) -> Self::Output {
        // TODO sync with typed_datex_values
        match (self.inner, rhs.inner) {
            (DatexValueInner::Text(text), other)
            | (other, DatexValueInner::Text(text)) => {
                let other =
                    try_cast_to_value_dyn::<Text>(other.to_dyn()).ok()?;
                let text = text.add(other);
                Some(text.as_datex_value())
            }
            (DatexValueInner::I8(lhs), DatexValueInner::I8(rhs)) => {
                Some(lhs.add(rhs).as_datex_value())
            }
            _ => None,
        }
    }
}

impl Not for DatexValue {
    type Output = Option<DatexValue>;

    fn not(self) -> Self::Output {
        if let Ok(typed) = self.try_cast_to_typed::<Bool>() {
            Some(DatexValue::from(!typed.inner().0))
        } else {
            None
        }
    }
}

impl<T> AddAssign<T> for DatexValue
where
    DatexValue: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        let rhs: DatexValue = rhs.into();
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

impl std::fmt::Debug for DatexValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_dyn())
    }
}

impl std::fmt::Display for DatexValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_dyn().fmt(f)
    }
}

impl<T> From<Option<T>> for DatexValue
where
    T: Into<DatexValue>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),

            // FIXME we should not use the type inference here
            None => DatexValue::null(),
        }
    }
}

impl<'de> Deserialize<'de> for DatexValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let intermediate = SerializableDatexValue::deserialize(deserializer)?;
        DatexValue::try_from(intermediate).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        datex_array, datex_values_new::array::DatexArray, logger::init_logger,
    };
    use log::{debug, info};

    #[test]
    fn new_addition_assignments() {
        let mut x = DatexValue::from(42);
        let y = DatexValue::from(27);

        x += y.clone();
        assert_eq!(x.get_type(), DatexType::I8);
        assert_eq!(x, DatexValue::from(69));
    }

    #[test]
    fn new_additions() {
        let x = DatexValue::from(42);
        let y = DatexValue::from(27);

        let z = (x.clone() + y.clone()).unwrap();
        assert_eq!(z.get_type(), DatexType::I8);
        assert_eq!(z, DatexValue::from(69));
    }

    #[test]
    fn array() {
        init_logger();
        let a = DatexValue::from(vec![
            DatexValue::from("42"),
            DatexValue::from(42),
            DatexValue::from(true),
        ]);

        let mut a = a.cast_to_typed::<DatexArray>();
        a.push(DatexValue::from(42));
        a.push(4);
        a += 42;
        a += DatexArray::from(vec!["inner", "array"]);
        let a: DatexArray = a.into_inner();

        assert_eq!(a.length(), 7);
        debug!("Array: {}", a);

        let b = DatexArray::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.length(), 11);
        assert_eq!(b.get_type(), DatexType::Array);

        let c = datex_array![1, "test", 3, true, false];
        assert_eq!(c.length(), 5);
        assert_eq!(c[0], 1.into());
        assert_eq!(c[1], "test".into());
        assert_eq!(c[2], 3.into());

        //c.insert(0, 1.into());
        debug!("Array: {}", c);
    }

    #[test]
    fn serialize() {
        init_logger();
        test_serialize_and_deserialize(DatexValue::from(42));
        test_serialize_and_deserialize(DatexValue::from("Hello World!"));
        test_serialize_and_deserialize(DatexValue::from(true));
        test_serialize_and_deserialize(DatexValue::from(false));
        test_serialize_and_deserialize(DatexValue::null());
        test_serialize_and_deserialize(DatexValue::from(0));
        test_serialize_and_deserialize(DatexValue::from(1));
    }

    #[test]
    fn typed_boolean() {
        init_logger();
        let a = TypedDatexValue::from(true);
        let b = TypedDatexValue::from(false);

        assert_eq!(a.get_type(), DatexType::Bool);
        assert_eq!(b.get_type(), DatexType::Bool);
        assert_ne!(a, b);
        assert_eq!(b, false);
        assert_eq!(!a, b);

        let mut a = TypedDatexValue::from(true);
        let b = TypedDatexValue::from(false);
        a.toggle();
        assert_eq!(a, b);
    }

    #[test]
    fn boolean() {
        init_logger();
        let a = DatexValue::from(true);
        let b = DatexValue::from(false);
        let c = DatexValue::from(false);
        assert_eq!(a.get_type(), DatexType::Bool);
        assert_eq!(b.get_type(), DatexType::Bool);
        assert!(a != b);
        assert!(b == c);

        let d = (!b.clone()).unwrap();
        assert_eq!(a, d);

        // We can't add two booleans together, so this should return None
        let a_plus_b = a.clone() + b.clone();
        assert!(a_plus_b.is_none());
    }

    #[test]
    fn type_casting_into() {
        init_logger();
        let a: TypedDatexValue<Text> =
            DatexValue::from("42").try_into().unwrap();
        assert_eq!(a.get_type(), DatexType::Text);

        let a: TypedDatexValue<Text> = DatexValue::from(42).try_into().unwrap();
        assert_eq!(a.get_type(), DatexType::Text);

        // This should fail because we are trying to cast a null value into a TypedDatexValue<Text>
        let a: Result<TypedDatexValue<Text>, _> = DatexValue::null().try_into();
        assert!(a.is_err());
    }

    #[test]
    fn test_cast_type() {
        init_logger();
        let a = DatexValue::from(42);
        let b = a.cast_to(DatexType::Text).unwrap();
        assert_eq!(b.get_type(), DatexType::Text);

        let c = a.cast_to_typed::<I8>();
        assert_eq!(c.into_erased(), DatexValue::from(42));

        let d = a.cast_to_typed::<Text>();
        assert_eq!(d.get_type(), DatexType::Text);
        assert_eq!(d.as_str(), "42");
    }

    #[test]
    fn test_infer_type() {
        init_logger();
        let a = TypedDatexValue::from(42);
        let b = TypedDatexValue::from(11);
        let c = TypedDatexValue::from("11");
        assert_eq!(c.length(), 2);

        assert_eq!(a.get_type(), DatexType::I8);
        assert_eq!(b.get_type(), DatexType::I8);

        let a_plus_b = a.clone() + b.clone();
        assert_eq!(a_plus_b.clone().get_type(), DatexType::I8);
        assert_eq!(a_plus_b.clone().into_erased(), DatexValue::from(53));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b.clone());
    }

    #[test]
    fn test_null() {
        init_logger();

        let null_value = DatexValue::null();
        assert_eq!(null_value.get_type(), DatexType::Null);
        assert_eq!(null_value.to_string(), "null");

        let maybe_value: Option<i8> = None;
        let null_value = DatexValue::from(maybe_value);
        assert_eq!(null_value.get_type(), DatexType::Null);
        assert_eq!(null_value.to_string(), "null");
    }

    #[test]
    fn test_text() {
        init_logger();
        let a = TypedDatexValue::from("Hello");
        assert_eq!(a, "Hello");
        assert_eq!(a.get_type(), DatexType::Text);
        assert_eq!(a.length(), 5);
        assert_eq!(a.to_string(), "\"Hello\"");
        assert_eq!(a.as_str(), "Hello");
        assert_eq!(a.to_uppercase(), "HELLO".into());
        assert_eq!(a.to_lowercase(), "hello".into());

        let b = &mut TypedDatexValue::from("World");
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
        let mut a: TypedDatexValue<Text> = TypedDatexValue::from("Hello");
        a += " World"; // see (#2)
        a += TypedDatexValue::from("4"); // Is typesafe
        a += 2;
        a += TypedDatexValue::from(42); // Is typesafe see (#1)
                                        // We won't allow this: `a += TypedDatexValue::from(true);`
        a += DatexValue::from("!"); // Might throw if the assignment would be incompatible.
        assert_eq!(a.length(), 16);
        assert_eq!(a.as_str(), "Hello World4242!");
    }

    #[test]
    fn test_test_assign2() {
        init_logger();
        let mut a = TypedDatexValue::from("Hello");
        a += " World";
        a += DatexValue::from("!");

        assert_eq!(a.length(), 12);
        assert_eq!(a.as_str(), "Hello World!");

        a += 42;

        assert_eq!(a.length(), 14);
        assert_eq!(a.as_str(), "Hello World!42");

        let mut b = DatexValue::from("Hello");
        b += " World ";
        b += TypedDatexValue::from(42);
        b += DatexValue::from("!");

        let b = b.cast_to_typed::<Text>();

        info!("{}", b);
        assert_eq!(b.length(), 15);
        assert_eq!(b.as_str(), "Hello World 42!");
    }

    #[test]
    fn test_typed_addition() {
        init_logger();
        let a = TypedDatexValue::from(42);
        let b = TypedDatexValue::from(27);

        assert_eq!(a, 42);
        assert_eq!(b, 27);

        assert_eq!(a.get_type(), DatexType::I8);
        assert_eq!(b.get_type(), DatexType::I8);

        let a_plus_b = a.clone() + b.clone();

        assert_eq!(a_plus_b.get_type(), DatexType::I8);

        assert_eq!(a_plus_b, TypedDatexValue::from(69));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn test_addition() {
        init_logger();
        let a = DatexValue::from(42);
        let b = DatexValue::from(27);

        assert_eq!(a.get_type(), DatexType::I8);
        assert_eq!(b.get_type(), DatexType::I8);

        let a_plus_b = (a.clone() + b.clone()).unwrap();

        assert_eq!(a_plus_b.get_type(), DatexType::I8);

        assert_eq!(a_plus_b, DatexValue::from(69));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn test_string_concatenation() {
        init_logger();
        let a = DatexValue::from("Hello ");
        let b = DatexValue::from(42i8);

        assert_eq!(a.get_type(), DatexType::Text);
        assert_eq!(b.get_type(), DatexType::I8);

        let a_plus_b = a.clone() + b.clone();
        let b_plus_a = b.clone() + a.clone();

        info!("a: {}", a);
        info!("b: {}", b);
        info!("a + b: {:?}", b_plus_a);

        return;

        let a_plus_b = a_plus_b.unwrap();
        let b_plus_a = b_plus_a.unwrap();

        assert_eq!(a_plus_b.get_type(), DatexType::Text);
        assert_eq!(b_plus_a.get_type(), DatexType::Text);

        assert_eq!(a_plus_b, DatexValue::from("Hello 42"));
        assert_eq!(b_plus_a, DatexValue::from("42Hello "));

        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
        info!("{} + {} = {}", b.clone(), a.clone(), b_plus_a);
    }

    fn serialize_datex_value(value: &DatexValue) -> String {
        let res = serde_json::to_string(value).unwrap();
        info!("Serialized DatexValue: {}", res);
        res
    }
    fn deserialize_datex_value(json: &str) -> DatexValue {
        let res = serde_json::from_str(json).unwrap();
        info!("Deserialized DatexValue: {}", res);
        res
    }
    fn test_serialize_and_deserialize(value: DatexValue) {
        let json = serialize_datex_value(&value);
        let deserialized = deserialize_datex_value(&json);
        assert_eq!(value, deserialized);
    }
}

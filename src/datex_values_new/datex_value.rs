use std::any::Any;
use std::fmt::{self, Display};
use std::ops::{Add, AddAssign};

use super::datex_type::DatexType;
use super::null::Null;
use super::primitive::PrimitiveI8;
use super::text::Text;
use super::typed_datex_value::TypedDatexValue;

pub trait AddAssignable: Any + Send + Sync {
    fn add_assign_boxed(&mut self, other: &dyn Value) -> Option<()>;
}

pub trait Value: Display + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn cast_to(&self, target: DatexType) -> Option<DatexValue>;
    fn as_datex_value(&self) -> DatexValue;
    fn get_type(&self) -> DatexType;
    fn add(&self, other: &dyn Value) -> Option<DatexValue>;
    fn static_type() -> DatexType
    where
        Self: Sized;

    fn as_add_assignable_mut(&mut self) -> Result<&mut dyn AddAssignable, ()> {
        Err(())
    }
}

use std::sync::Arc;

#[derive(Clone)]
pub struct DatexValue(pub Arc<dyn Value>);

impl<T: Value + 'static> From<TypedDatexValue<T>> for DatexValue {
    fn from(typed: TypedDatexValue<T>) -> Self {
        DatexValue(Arc::new(typed.0))
    }
}

impl DatexValue {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }
    pub fn boxed<V: Value + 'static>(v: V) -> Self {
        DatexValue(Arc::new(v))
    }

    pub fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        self.0.cast_to(target)
    }
    pub fn try_cast_to_typed<T: Value + Clone + 'static>(
        &self,
    ) -> Result<TypedDatexValue<T>, ()> {
        let casted = self.cast_to(T::static_type()).ok_or(())?;
        let casted = casted
            .0
            .as_any()
            .downcast_ref::<T>()
            .map(|v| TypedDatexValue(v.clone()));
        casted.ok_or(())
    }

    pub fn cast_to_typed<T: Value + Clone + 'static>(
        &self,
    ) -> TypedDatexValue<T> {
        return self.try_cast_to_typed::<T>().unwrap_or_else(|_| {
            panic!("Failed to cast to type: {:?}", T::static_type())
        });
    }

    pub fn get_type(&self) -> DatexType {
        self.0.get_type()
    }
    pub fn concatenate(&self, other: &dyn Value) -> Option<DatexValue> {
        let other_casted = other.cast_to(DatexType::Text)?;
        let other_value = other_casted.0.as_any().downcast_ref::<Text>()?;
        Some(DatexValue::boxed(Text(format!(
            "{}{}",
            self.0, other_value.0
        ))))
    }
}
impl DatexValue {
    pub fn null() -> Self {
        DatexValue::boxed(Null)
    }
}
impl PartialEq for DatexValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.get_type() == other.0.get_type()
            && self.0.as_any().type_id() == other.0.as_any().type_id()
            && match self.0.get_type() {
                DatexType::Text => {
                    let a = self.0.as_any().downcast_ref::<Text>();
                    let b = other.0.as_any().downcast_ref::<Text>();
                    a == b
                }
                DatexType::PrimitiveI8 => {
                    let a = self.0.as_any().downcast_ref::<PrimitiveI8>();
                    let b = other.0.as_any().downcast_ref::<PrimitiveI8>();
                    a == b
                }
                _ => false,
            }
    }
}

impl Add for DatexValue {
    type Output = DatexValue;

    fn add(self, rhs: DatexValue) -> DatexValue {
        self.0.add(rhs.0.as_ref()).unwrap_or_else(|| {
            panic!("Unsupported addition: {} + {}", self, rhs)
        })
    }
}

impl<T> AddAssign<T> for DatexValue
where
    DatexValue: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        let rhs_val = DatexValue::from(rhs);
        let rhs_ref = rhs_val.0.as_ref();

        let inner_mut =
            Arc::get_mut(&mut self.0).expect("Cannot mutate shared DatexValue");
        if let Ok(addable) = inner_mut.as_add_assignable_mut() {
            if addable.add_assign_boxed(rhs_ref).is_some() {
                return;
            }
        }
        panic!("Cannot mutate shared DatexValue");
    }
}

impl std::fmt::Debug for DatexValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for DatexValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for DatexValue {
    fn from(s: &str) -> Self {
        DatexValue::boxed(Text(s.to_string()))
    }
}

impl From<String> for DatexValue {
    fn from(s: String) -> Self {
        DatexValue::boxed(Text(s))
    }
}
impl From<i8> for DatexValue {
    fn from(v: i8) -> Self {
        DatexValue::boxed(PrimitiveI8(v))
    }
}

impl<T> From<Option<T>> for DatexValue
where
    T: Into<DatexValue>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => DatexValue::null(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::logger::init_logger;
    use log::info;

    #[test]
    fn test_cast_type() {
        init_logger();
        let a = DatexValue::from(42);
        let b = a.cast_to(DatexType::Text).unwrap();
        assert_eq!(b.get_type(), DatexType::Text);

        let c = a.cast_to_typed::<PrimitiveI8>();
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

        assert_eq!(a.get_type(), DatexType::PrimitiveI8);
        assert_eq!(b.get_type(), DatexType::PrimitiveI8);

        let a_plus_b = a.clone() + b.clone();
        assert_eq!(a_plus_b.clone().get_type(), DatexType::PrimitiveI8);
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
    fn test_test_assign() {
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
    fn test_addition() {
        init_logger();
        let a = DatexValue::from(42);
        let b = DatexValue::from(27);

        assert_eq!(a.get_type(), DatexType::PrimitiveI8);
        assert_eq!(b.get_type(), DatexType::PrimitiveI8);

        let a_plus_b = a.clone() + b.clone();
        assert_eq!(a_plus_b.get_type(), DatexType::PrimitiveI8);

        assert_eq!(a_plus_b, DatexValue::from(69));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn test_string_concatenation() {
        init_logger();
        let a = DatexValue::from("Hello ");
        let b = DatexValue::from(42i8);

        assert_eq!(a.get_type(), DatexType::Text);
        assert_eq!(b.get_type(), DatexType::PrimitiveI8);

        let a_plus_b = a.clone() + b.clone();
        let b_plus_a = b.clone() + a.clone();

        assert_eq!(a_plus_b.get_type(), DatexType::Text);
        assert_eq!(b_plus_a.get_type(), DatexType::Text);

        assert_eq!(a_plus_b, DatexValue::from("Hello 42"));
        assert_eq!(b_plus_a, DatexValue::from("42Hello "));

        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
        info!("{} + {} = {}", b.clone(), a.clone(), b_plus_a);
    }
}

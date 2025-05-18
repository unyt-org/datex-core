use std::fmt::{self, Display};
use std::ops::Add;

use super::null::Null;
use super::primitive::Primitive;
#[derive(Debug, Clone, PartialEq)]
pub enum DatexValue {
    Null,
    Text(String),
    Primitive(Primitive),
    Composite(Vec<DatexValue>),
}
impl From<&str> for DatexValue {
    fn from(s: &str) -> Self {
        DatexValue::Text(s.to_string())
    }
}

impl From<String> for DatexValue {
    fn from(s: String) -> Self {
        DatexValue::Text(s)
    }
}

impl From<i8> for DatexValue {
    fn from(v: i8) -> Self {
        DatexValue::Primitive(Primitive::I8(v))
    }
}

impl From<u8> for DatexValue {
    fn from(v: u8) -> Self {
        DatexValue::Primitive(Primitive::U8(v))
    }
}
impl From<Null> for DatexValue {
    fn from(_: Null) -> Self {
        DatexValue::Null
    }
}

impl Display for DatexValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DatexValue::Null => write!(f, "null"),
            DatexValue::Text(s) => write!(f, "\"{}\"", s),
            DatexValue::Primitive(p) => write!(f, "{}", p),
            DatexValue::Composite(values) => {
                for val in values {
                    write!(f, "{}", val)?;
                }
                Ok(())
            }
        }
    }
}

impl DatexValue {
    pub fn coerce_to_string(&self) -> Option<String> {
        match self {
            DatexValue::Null => Some("null".to_string()),
            DatexValue::Text(s) => Some(s.clone()),
            DatexValue::Primitive(p) => Some(format!("{}", p)),
            DatexValue::Composite(values) => {
                let mut result = String::new();
                for v in values {
                    result.push_str(&v.coerce_to_string()?);
                }
                Some(result)
            }
        }
    }

    pub fn concat<I: IntoIterator<Item = DatexValue>>(values: I) -> Self {
        DatexValue::Composite(values.into_iter().collect())
    }
    pub fn cast_to_string(&self) -> Option<DatexValue> {
        self.coerce_to_string().map(DatexValue::Text)
    }

    pub fn cast_to_primitive_i8(&self) -> Option<DatexValue> {
        match self {
            DatexValue::Primitive(Primitive::I8(v)) => {
                Some(DatexValue::Primitive(Primitive::I8(*v)))
            }
            DatexValue::Text(s) => s
                .parse::<i8>()
                .ok()
                .map(|v| DatexValue::Primitive(Primitive::I8(v))),
            _ => None,
        }
    }
}

impl Add for DatexValue {
    type Output = DatexValue;

    fn add(self, rhs: DatexValue) -> Self::Output {
        match (self, rhs) {
            (DatexValue::Text(a), b) => {
                if let Some(b_str) = b.coerce_to_string() {
                    DatexValue::Text(a + &b_str)
                } else {
                    DatexValue::Null
                }
            }
            (a, DatexValue::Text(b)) => {
                if let Some(a_str) = a.coerce_to_string() {
                    DatexValue::Text(a_str + &b)
                } else {
                    DatexValue::Null
                }
            }
            (a, b) => DatexValue::concat(vec![a, b]),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::logger::init_logger;
    use log::info;

    #[test]
    fn test2() {
        init_logger();
        let a = DatexValue::from("Hello ");
        let b = DatexValue::from(42i8);
        let c = DatexValue::from(10u8);
        let d = DatexValue::from(Null);

        let result1 = a.clone() + b.clone();
        let result2 = b.clone() + c;
        let result3 = result1.clone() + d;
        let result4 = b.clone().cast_to_string();

        info!("result1: {}", result1);
        info!("result2: {}", result2);
        info!("result3: {}", result3);
        info!("result4: {:?}", result4);
    }

    #[test]
    fn test_datex_value() {
        let value1 = DatexValue::Text("Hello".to_string());
        let value2 = DatexValue::Text("World".to_string());
        let value3 = value1.clone() + value2.clone();
        assert_eq!(value3.coerce_to_string(), Some("HelloWorld".to_string()));

        let value4 = DatexValue::Primitive(Primitive::I8(42));
        let value5 = DatexValue::Primitive(Primitive::U8(100));
        let value6 = value4.clone() + value5.clone();
        assert_eq!(value6.coerce_to_string(), Some("42100".to_string()));
    }
}

use crate::values::{
    core_value::CoreValue,
    core_values::{
        decimal::{Decimal, typed_decimal::TypedDecimal},
        integer::typed_integer::TypedInteger,
    },
    value_container::error::ValueError,
};
use core::{ops::Add, result::Result};

impl Add for &CoreValue {
    type Output = Result<CoreValue, ValueError>;
    fn add(self, rhs: &CoreValue) -> Self::Output {
        println!("Adding {:?} and {:?}", self, rhs);
        match (self, rhs) {
            // x + text or text + x (order does not matter)
            (CoreValue::Text(text), other) => {
                let other = other.cast_to_text();
                return Ok(CoreValue::Text(text + other));
            }
            (other, CoreValue::Text(text)) => {
                let other = other.cast_to_text();
                return Ok(CoreValue::Text(other + text));
            }

            // same type additions
            (CoreValue::TypedInteger(lhs), CoreValue::TypedInteger(rhs)) => {
                return Ok(CoreValue::TypedInteger(
                    (lhs + rhs).ok_or(ValueError::IntegerOverflow)?,
                ));
            }
            (CoreValue::Integer(lhs), CoreValue::Integer(rhs)) => {
                return Ok(CoreValue::Integer(lhs + rhs));
            }
            (CoreValue::TypedDecimal(lhs), CoreValue::TypedDecimal(rhs)) => {
                return Ok(CoreValue::TypedDecimal(lhs + rhs));
            }
            (CoreValue::Decimal(lhs), CoreValue::Decimal(rhs)) => {
                return Ok(CoreValue::Decimal(lhs + rhs));
            }
            (CoreValue::Native(lhs), CoreValue::Native(rhs)) => {
                return Ok(CoreValue::Native(
                    (lhs + rhs).ok_or(ValueError::InvalidOperation)?,
                ));
            }

            _ => {}
        }

        // other cases
        match &self {
            // integer
            CoreValue::Integer(lhs) => match &rhs {
                CoreValue::TypedInteger(rhs) => {
                    Ok(CoreValue::Integer(lhs + &rhs.as_integer()))
                }
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        ._cast_to_integer_internal()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::Integer(lhs + &integer.as_integer()))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::Integer(lhs + &integer.as_integer()))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // typed integer
            CoreValue::TypedInteger(lhs) => match &rhs {
                CoreValue::Integer(_rhs) => {
                    core::todo!(
                        "#317 TypedInteger + Integer not implemented yet"
                    );
                    //Ok(CoreValue::TypedInteger(lhs.as_integer() + rhs.clone()))
                }
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        ._cast_to_integer_internal()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::TypedInteger(
                        (lhs + &integer).ok_or(ValueError::IntegerOverflow)?,
                    ))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::TypedInteger(
                        (lhs + &integer).ok_or(ValueError::IntegerOverflow)?,
                    ))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // decimal
            CoreValue::Decimal(lhs) => match rhs {
                CoreValue::TypedDecimal(rhs) => {
                    Ok(CoreValue::Decimal(lhs + &Decimal::from(rhs)))
                }
                CoreValue::TypedInteger(rhs) => {
                    let decimal = Decimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::Decimal(lhs + &decimal))
                }
                CoreValue::Integer(rhs) => {
                    let decimal = Decimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::Decimal(lhs + &decimal))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // typed decimal
            CoreValue::TypedDecimal(lhs) => match rhs {
                CoreValue::Decimal(rhs) => Ok(CoreValue::TypedDecimal(
                    lhs + &TypedDecimal::Decimal(rhs.clone()),
                )),
                CoreValue::TypedInteger(rhs) => {
                    let decimal = TypedDecimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::TypedDecimal(lhs + &decimal))
                }
                CoreValue::Integer(rhs) => {
                    let decimal = TypedDecimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::TypedDecimal(lhs + &decimal))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            _ => Err(ValueError::InvalidOperation),
        }
    }
}

impl Add for CoreValue {
    type Output = Result<CoreValue, ValueError>;
    fn add(self, rhs: CoreValue) -> Self::Output {
        (&self).add(&rhs)
    }
}

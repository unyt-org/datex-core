use std::fmt::Display;

use num_traits::Float;
use ordered_float::OrderedFloat;

use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;

// TODO: currently not required
pub fn smallest_fitting_float(value: f64) -> TypedDecimal {
    if value.is_nan()
        || value.is_infinite()
        || (value >= f32::MIN as f64 && value <= f32::MAX as f64)
    {
        TypedDecimal::F32(OrderedFloat(value as f32))
    }
    // otherwise use f64
    else {
        TypedDecimal::F64(OrderedFloat(value))
    }
}

pub fn decimal_to_string<T: Float + Display + std::fmt::LowerExp>(
    value: T,
    json_compatible: bool,
) -> String {
    if value.is_nan() {
        if json_compatible {
            "NaN".to_string()
        } else {
            "nan".to_string()
        }
    } else if value.is_infinite() {
        format!(
            "{}{}",
            if value.is_sign_positive() { "" } else { "-" },
            if json_compatible {
                "Infinity".to_string()
            } else {
                "infinity".to_string()
            }
        )
    }
    // e notation for large or small numbers
    else if value.abs() > T::from(1e20).unwrap() || value.abs() < T::from(1e-7).unwrap() {
        format!("{value:.e}")
    }
    else if value.fract() == T::zero() {
        format!("{value:.1}")
    }
    else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use ordered_float::OrderedFloat;
    use super::*;

    #[test]
    fn test_smallest_fitting_float() {
        assert_eq!(
            smallest_fitting_float(1.0),
            TypedDecimal::F32(OrderedFloat(1.0))
        );
        assert_eq!(
            smallest_fitting_float(1.5),
            TypedDecimal::F32(OrderedFloat(1.5))
        );
        assert_eq!(
            smallest_fitting_float(1e200),
            TypedDecimal::F64(OrderedFloat(1e200))
        );
        assert_eq!(
            smallest_fitting_float(f64::NAN).is_nan(),
            TypedDecimal::F32(OrderedFloat(f32::NAN)).is_nan()
        );
    }

    #[test]
    fn test_format_e_notation() {
        let value = 1e20;
        let formatted = decimal_to_string(value, false);
        assert_eq!(formatted, "100000000000000000000.0");

        let value = 1e-7;
        let formatted = decimal_to_string(value, false);
        assert_eq!(formatted, "0.0000001");

        let value = 1e-8;
        let formatted = decimal_to_string(value, false);
        assert_eq!(formatted, "1e-8");

        let value = 1e21;
        let formatted = decimal_to_string(value, false);
        assert_eq!(formatted, "1e21");
    }
}

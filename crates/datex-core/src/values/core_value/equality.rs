use crate::{
    traits::{dyn_eq::DynEq, structural_eq::StructuralEq, value_eq::ValueEq},
    values::{
        core_value::CoreValue,
        core_values::{
            decimal::typed_decimal::TypedDecimal,
            integer::typed_integer::TypedInteger,
        },
    },
};

impl StructuralEq for CoreValue {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CoreValue::Boolean(a), CoreValue::Boolean(b)) => {
                a.structural_eq(b)
            }

            // Integers
            (CoreValue::Integer(a), CoreValue::Integer(b)) => {
                a.structural_eq(b)
            }

            // TypedIntegers
            (CoreValue::TypedInteger(a), CoreValue::TypedInteger(b)) => {
                a.structural_eq(b)
            }

            // Integers + TypedIntegers
            (CoreValue::Integer(a), CoreValue::TypedInteger(b))
            | (CoreValue::TypedInteger(b), CoreValue::Integer(a)) => {
                TypedInteger::IBig(a.clone()).structural_eq(b)
            }

            // Decimals
            (CoreValue::Decimal(a), CoreValue::Decimal(b)) => {
                a.structural_eq(b)
            }

            // TypedDecimals
            (CoreValue::TypedDecimal(a), CoreValue::TypedDecimal(b)) => {
                a.structural_eq(b)
            }

            // Decimal + TypedDecimal
            (CoreValue::Decimal(a), CoreValue::TypedDecimal(b))
            | (CoreValue::TypedDecimal(b), CoreValue::Decimal(a)) => {
                TypedDecimal::Decimal(a.clone()).structural_eq(b)
            }

            (CoreValue::Text(a), CoreValue::Text(b)) => a.structural_eq(b),
            (CoreValue::Null, CoreValue::Null) => true,
            (CoreValue::Endpoint(a), CoreValue::Endpoint(b)) => {
                a.structural_eq(b)
            }
            (CoreValue::List(a), CoreValue::List(b)) => a.structural_eq(b),
            (CoreValue::Map(a), CoreValue::Map(b)) => a.structural_eq(b),
            (CoreValue::Type(a), CoreValue::Type(b)) => a.structural_eq(b),
            (CoreValue::Callable(a), CoreValue::Callable(b)) => {
                a.structural_eq(b)
            }

            (CoreValue::Range(a), CoreValue::Range(b)) => {
                a.start.structural_eq(&b.start) && a.end.structural_eq(&b.end)
            }
            _ => false,
        }
    }
}

/// Value equality corresponds to partial equality for all values,
/// except for decimals, where partial equality is also given for NaN values and +0.0 and -0.0.
/// Therefore, we ValueEq is used instead for decimals
impl ValueEq for CoreValue {
    fn value_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CoreValue::Decimal(a), CoreValue::Decimal(b)) => a.value_eq(b),
            (CoreValue::TypedDecimal(a), CoreValue::TypedDecimal(b)) => {
                a.value_eq(b)
            }
            _ => self == other,
        }
    }
}

impl PartialEq for CoreValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CoreValue::Uninitialized, CoreValue::Uninitialized) => true,
            (CoreValue::Null, CoreValue::Null) => true,
            (CoreValue::Boolean(b1), CoreValue::Boolean(b2)) => b1 == b2,
            (CoreValue::Integer(i1), CoreValue::Integer(i2)) => i1 == i2,
            (CoreValue::TypedInteger(ti1), CoreValue::TypedInteger(ti2)) => {
                ti1 == ti2
            }
            (CoreValue::Decimal(d1), CoreValue::Decimal(d2)) => d1 == d2,
            (CoreValue::TypedDecimal(td1), CoreValue::TypedDecimal(td2)) => {
                td1 == td2
            }
            (CoreValue::Text(t1), CoreValue::Text(t2)) => t1 == t2,
            (CoreValue::Endpoint(e1), CoreValue::Endpoint(e2)) => e1 == e2,
            (CoreValue::List(l1), CoreValue::List(l2)) => l1 == l2,
            (CoreValue::Map(m1), CoreValue::Map(m2)) => m1 == m2,
            (CoreValue::Type(t1), CoreValue::Type(t2)) => t1 == t2,
            (
                CoreValue::EntityTypeDefinition(etd1),
                CoreValue::EntityTypeDefinition(etd2),
            ) => etd1 == etd2,
            (CoreValue::Callable(c1), CoreValue::Callable(c2)) => c1 == c2,
            (CoreValue::Range(r1), CoreValue::Range(r2)) => r1 == r2,
            (CoreValue::Box(b1), CoreValue::Box(b2)) => *b1 == *b2,
            (CoreValue::Native(n1), CoreValue::Native(n2)) => {
                n1.value.dyn_eq(&*n2.value)
            }
            (CoreValue::Native(n), other) => other.dyn_eq_native(&*n.value),
            (other, CoreValue::Native(n)) => other.dyn_eq_native(&*n.value),
            _ => false,
        }
    }
}

impl Eq for CoreValue {}
impl CoreValue {
    fn dyn_eq_native(&self, native: &dyn DynEq) -> bool {
        match self {
            CoreValue::Boolean(v) => native.dyn_eq(v),
            CoreValue::Integer(v) => native.dyn_eq(v),
            CoreValue::TypedInteger(v) => native.dyn_eq(v),
            CoreValue::Decimal(v) => native.dyn_eq(v),
            CoreValue::TypedDecimal(v) => native.dyn_eq(v),
            CoreValue::Text(v) => native.dyn_eq(v),
            CoreValue::Endpoint(v) => native.dyn_eq(v),
            CoreValue::List(v) => native.dyn_eq(v),
            CoreValue::Map(v) => native.dyn_eq(v),
            CoreValue::Type(v) => native.dyn_eq(v),
            CoreValue::Callable(v) => native.dyn_eq(v),
            CoreValue::Range(v) => native.dyn_eq(v),
            CoreValue::Null => native.dyn_eq(&()), //FIXME or false?
            CoreValue::Uninitialized => todo!(),
            CoreValue::EntityTypeDefinition(entity_type_definition) => todo!(),
            CoreValue::Box(value_container) => value_container.dyn_eq(native), // FIXME
            CoreValue::Native(native_core_value) => {
                unreachable!("covered above")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preludes::derive::CoreValue,
        values::core_values::{
            boolean::Boolean, endpoint::Endpoint,
            integer::typed_integer::TypedInteger, native::NativeCoreValue,
        },
    };

    #[test]
    fn native_eq() {
        let native = CoreValue::Native(NativeCoreValue::new(Boolean(true)));
        let non_native = CoreValue::Boolean(Boolean(true));
        assert_eq!(native, non_native);

        let native =
            CoreValue::Native(NativeCoreValue::new(Endpoint::new("@jonas")));
        let non_native = CoreValue::Endpoint(Endpoint::new("@jonas"));
        assert_eq!(native, non_native);

        let native = CoreValue::native(TypedInteger::I8(42));
        let non_native = CoreValue::TypedInteger(TypedInteger::I8(42));
        assert_eq!(native, non_native);
    }

    // FIXME
    #[test]
    fn native_eq_rust() {
        let native = CoreValue::native(42);
        let non_native = CoreValue::TypedInteger(TypedInteger::I8(42));
        assert_eq!(native, non_native);
    }

    #[test]
    fn native_ne() {
        let native = CoreValue::Native(NativeCoreValue::new(Boolean(true)));
        let value = CoreValue::Boolean(Boolean(false));

        assert_ne!(native, value);
        assert_ne!(value, native);
    }
}

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    hash::Hasher,
};

use serde::{Deserialize, Serialize};

use crate::values::{
    core_values::r#type::path::TypePath, datex_type::CoreValueType,
    value_container::ValueContainer,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeDescriptor {
    /// A reference to a type path (e.g. "core:integer")
    Reference(TypePath),

    /// A core primitive (integer, integer/u8, ...)
    Core(CoreValueType),

    // var User = math.radom() > 0.5 ?
    //     { age: 42 } & { x: 2 + 5}
    //     { x: 5}

    // var x = i ? 2 : integer;
    // infered type: 2 | type<integer>
    // [1,2,3,4,5,6,63] -> Array[1,2,3]
    // [1,2,3,4,5,6,63] -> Array[u8] ?
    // &mut [1,2,3,4,5,6,63] -> Array[1,2,3]
    // &[1,2,3, $34334] -> Array<u8>

    // const x = 0.repeat(100000) // 0[]

    // type x =( { age: 42 } & { x: 2 + 5}) | { x: 5}

    // pure function mytype (x: any) (
    //     var x = User
    //
    //     x
    // )
    // type x = 42;
    // function myfunction(x: Type<X>) -> Type<Y> {
    //     x
    // }
    // var x = User;
    // type User2 = {age: integer/u8}
    // var y = new User {age: 5};
    // typeof x == Type<User>
    // typeof y == User
    // y matches User // false
    // User2 matches User { age: integer } // true

    // FIXME enum function
    /// A literal type (e.g. `"hello"`, `2`, `true`)
    Literal(ValueContainer),

    // /// A nominal type (referenced by name)
    // Nominal { name: String },
    /// A struct type { a: string, b: integer }
    Record(HashMap<String, TypeDescriptor>),

    /// A tuple type (A, B, C)
    Tuple(Vec<TypeDescriptor>),

    /// A union type (A | B | C)
    Union(Vec<TypeDescriptor>),
}

// User { x: 4}
use std::hash::Hash;
impl Hash for TypeDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TypeDescriptor::Reference(path) => path.hash(state),
            TypeDescriptor::Core(core_type) => core_type.hash(state),
            TypeDescriptor::Literal(value) => value.hash(state),
            TypeDescriptor::Record(fields) => {
                fields.iter().for_each(|(k, v)| {
                    k.hash(state);
                    v.hash(state);
                });
            }
            TypeDescriptor::Tuple(types) => types.hash(state),
            TypeDescriptor::Union(types) => types.hash(state),
        }
    }
}

impl TypeDescriptor {
    pub fn is_reference(&self) -> bool {
        matches!(self, TypeDescriptor::Reference(_))
    }
}
impl Display for TypeDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeDescriptor::Reference(path) => write!(f, "*{}", path.as_str()),
            TypeDescriptor::Core(core_type) => write!(f, "std::{}", core_type),
            TypeDescriptor::Literal(value) => write!(f, "{{{}}}", value),
            TypeDescriptor::Record(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", fields_str.join(", "))
            }
            TypeDescriptor::Tuple(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", types_str.join(", "))
            }
            TypeDescriptor::Union(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", types_str.join(" | "))
            }
        }
    }
}

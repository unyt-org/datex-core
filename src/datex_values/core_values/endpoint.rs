use crate::crypto::random;
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_value_trait::CoreValueTrait;
use crate::datex_values::traits::soft_eq::SoftEq;
use crate::datex_values::value_container::{ValueContainer, ValueError};
use crate::stdlib::fmt::{Debug, Display, Formatter};
use crate::stdlib::hash::Hash;
use crate::utils::buffers::buffer_to_hex;
use binrw::{BinRead, BinWrite};
use hex::decode;
// FIXME no-std
use crate::stdlib::str;
use std::cell::Ref;
use std::io::Cursor;
use std::str::FromStr;
use strum::Display;

#[derive(
    BinWrite, BinRead, Debug, Clone, Copy, Hash, PartialEq, Eq, Default,
)]
pub enum EndpointInstance {
    // targets any instance, but exactly one endpoint
    // syntax: @x/0000 == @x
    #[default]
    #[br(magic = 0u16)]
    #[bw(magic = 0u16)]
    Any,
    // targets all instances of the endpoint
    // syntax: @x/65535 == @x/*
    #[br(magic = 65535u16)]
    #[bw(magic = 65535u16)]
    All,
    // targets a specific instance of the endpoint
    // syntax: @x/[1-65534]
    Instance(u16),
}

impl EndpointInstance {
    pub fn new(instance: u16) -> EndpointInstance {
        match instance {
            0 => EndpointInstance::Any,
            65535 => EndpointInstance::All,
            _ => EndpointInstance::Instance(instance),
        }
    }
}

// 1 byte
#[derive(
    Debug, Hash, PartialEq, Eq, Clone, Copy, Default, BinWrite, BinRead,
)]
#[brw(repr(u8))]
pub enum EndpointType {
    #[default]
    Person = 0,
    Institution = 1,
    Anonymous = 2,
}

#[derive(BinWrite, BinRead, Debug, Clone, Hash, PartialEq, Eq)]
#[brw(little)]
pub struct Endpoint {
    // 1 byte type, 18 bytes name, 2 bytes instance
    pub type_: EndpointType,
    pub identifier: [u8; 18],
    pub instance: EndpointInstance,
}

// new into
impl<T: Into<ValueContainer>> TryFrom<Option<T>> for Endpoint {
    type Error = ValueError;
    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        if let Some(value) = value {
            let container: ValueContainer = value.into();
            if let Some(endpoint) = container.cast_to_endpoint() {
                return Ok(endpoint);
            }
        }
        Err(ValueError::TypeConversionError)
    }
}
// also for ref
// impl<T: Into<ValueContainer>> TryFrom<&Option<T>> for Endpoint {
//     type Error = ValueError;
//     fn try_from(value: &Option<T>) -> Result<Self, Self::Error> {
//         if let Some(value) = value {
//             let container: Ref<ValueContainer> = value.into();
//             if let Some(endpoint) = container.cast_to_endpoint() {
//                 return Ok(endpoint);
//             }
//         }
//         Err(ValueError::TypeConversionError)
//     }
// }

impl CoreValueTrait for Endpoint {}

impl SoftEq for Endpoint {
    fn soft_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Default for Endpoint {
    fn default() -> Self {
        Endpoint::LOCAL
    }
}

impl From<&str> for Endpoint {
    fn from(name: &str) -> Self {
        if let Ok(endpoint) = Endpoint::from_string(name) {
            return endpoint;
        }
        panic!("Failed to parse endpoint from string: {name}");
    }
}

// impl From<CoreValue> for Endpoint {
//     fn from(value: CoreValue) -> Self {
//         return value.cast_to_endpoint().unwrap();
//     }
// }

impl TryFrom<CoreValue> for Endpoint {
    type Error = ValueError;
    fn try_from(value: CoreValue) -> Result<Self, Self::Error> {
        if let Some(endpoint) = value.cast_to_endpoint() {
            return Ok(endpoint);
        }
        Err(ValueError::TypeConversionError)
    }
}

#[derive(PartialEq, Debug, Display)]
pub enum InvalidEndpointError {
    InvalidCharacters,
    MaxLengthExceeded,
    MinLengthNotMet,
    InvalidInstance,
    ReservedName,
}
#[derive(PartialEq, Debug)]
pub struct EndpointParsingError;

impl Endpoint {
    const PREFIX_PERSON: &'static str = "@";
    const PREFIX_INSTITUTION: &'static str = "@+";
    const PREFIX_ANONYMOUS: &'static str = "@@";

    const ALIAS_LOCAL: &'static str = "local";
    const ALIAS_ANY: &'static str = "any";

    // targets each endpoint, but exactly one instance
    // @@any == @@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF/0
    pub const ANY: Endpoint = Endpoint {
        type_: EndpointType::Anonymous,
        identifier: [255; 18],
        instance: EndpointInstance::Any,
    };

    // targets all instances of all endpoints
    // @@any/* == @@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF/*
    pub const ANY_ALL_INSTANCES: Endpoint = Endpoint {
        type_: EndpointType::Anonymous,
        identifier: [255; 18],
        instance: EndpointInstance::All,
    };

    // targets the local endpoint
    // @@local == @@000000000000000000000000000000000000/0
    pub const LOCAL: Endpoint = Endpoint {
        type_: EndpointType::Anonymous,
        identifier: [0; 18],
        instance: EndpointInstance::Any,
    };

    // targets all instances of the local endpoint
    // @@local/* == @@000000000000000000000000000000000000/*
    pub const LOCAL_ALL_INSTANCES: Endpoint = Endpoint {
        type_: EndpointType::Anonymous,
        identifier: [0; 18],
        instance: EndpointInstance::All,
    };

    // create a random anonymous endpoint (e.g. @@8D928D1F244C76289C8A558DCB6C9D82896F)
    pub fn random() -> Endpoint {
        Self::anonymous(Self::random_anonymous_id(), EndpointInstance::Any)
            .unwrap()
    }

    // create an anonymous endpoint (e.g. @@8D928D1F244C76289C8A558DCB6C9D82896F)
    pub fn anonymous(
        identifier: [u8; 18],
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointError> {
        // @@any endpoint
        if identifier == [255; 18] {
            return if instance == EndpointInstance::Any {
                Ok(Endpoint::ANY)
            } else if instance == EndpointInstance::All {
                Ok(Endpoint::ANY_ALL_INSTANCES)
            } else {
                Ok(Endpoint {
                    type_: EndpointType::Anonymous,
                    identifier,
                    instance,
                })
            };
        }

        // @@local endpoint
        if identifier == [0; 18] {
            return if instance == EndpointInstance::Any {
                Ok(Endpoint::LOCAL)
            } else if instance == EndpointInstance::All {
                Ok(Endpoint::LOCAL_ALL_INSTANCES)
            } else {
                Ok(Endpoint {
                    type_: EndpointType::Anonymous,
                    identifier,
                    instance,
                })
            };
        }

        Ok(Endpoint {
            type_: EndpointType::Anonymous,
            identifier,
            instance,
        })
    }

    // create alias endpoint (@person)
    pub fn person(
        name: &str,
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointError> {
        Self::named(name, instance, EndpointType::Person)
    }

    // create institution endpoint (@+institution)
    pub fn institution(
        name: &str,
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointError> {
        Self::named(name, instance, EndpointType::Institution)
    }

    // create endpoint from string (@person/42, @@local, @+unyt)
    fn from_string(name: &str) -> Result<Endpoint, InvalidEndpointError> {
        let name = name.to_string();
        if name
            == format!("{}{}", Endpoint::PREFIX_ANONYMOUS, Endpoint::ALIAS_ANY)
        {
            return Ok(Endpoint::ANY);
        } else if name
            == format!(
                "{}{}",
                Endpoint::PREFIX_ANONYMOUS,
                Endpoint::ALIAS_LOCAL
            )
        {
            return Ok(Endpoint::LOCAL);
        }

        let mut name_part = name.clone();
        let mut instance = EndpointInstance::Any;
        // check if instance is present
        if name.contains('/') {
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() != 2 {
                return Err(InvalidEndpointError::InvalidCharacters);
            }
            name_part = parts[0].to_string();
            let instance_str = parts[1];
            if instance_str == "*" {
                instance = EndpointInstance::All;
            } else {
                let instance_num = instance_str
                    .parse::<u16>()
                    .map_err(|_| InvalidEndpointError::InvalidInstance)?;
                instance = EndpointInstance::new(instance_num);
            }
        }

        let endpoint = match name_part {
            s if s.starts_with(&format!(
                "{}{}",
                Endpoint::PREFIX_ANONYMOUS,
                Endpoint::ALIAS_ANY
            )) =>
            {
                Ok(Endpoint {
                    type_: EndpointType::Anonymous,
                    identifier: [255u8; 18],
                    instance,
                })
            }
            s if s.starts_with(&format!(
                "{}{}",
                Endpoint::PREFIX_ANONYMOUS,
                Endpoint::ALIAS_LOCAL
            )) =>
            {
                Ok(Endpoint {
                    type_: EndpointType::Anonymous,
                    identifier: [0u8; 18],
                    instance,
                })
            }
            s if s.starts_with(Endpoint::PREFIX_ANONYMOUS) => {
                let s = s.trim_start_matches(Endpoint::PREFIX_ANONYMOUS);
                if s.len() < 18 * 2 {
                    return Err(InvalidEndpointError::MinLengthNotMet);
                } else if s.len() > 18 * 2 {
                    return Err(InvalidEndpointError::MaxLengthExceeded);
                }
                let bytes = decode(s)
                    .map_err(|_| InvalidEndpointError::InvalidCharacters)?;
                let byte_slice: &[u8] = &bytes;
                Endpoint::anonymous(byte_slice.try_into().unwrap(), instance)
            }
            s if s.starts_with(Endpoint::PREFIX_INSTITUTION) => {
                Endpoint::named(&s[2..], instance, EndpointType::Institution)
            }
            s if s.starts_with(Endpoint::PREFIX_PERSON) => {
                Endpoint::named(&s[1..], instance, EndpointType::Person)
            }
            _ => return Err(InvalidEndpointError::InvalidCharacters),
        };
        endpoint
    }

    // parse endpoint from binary
    pub fn from_binary(
        binary: [u8; 21],
    ) -> Result<Endpoint, EndpointParsingError> {
        let mut reader = Cursor::new(binary);
        let endpoint =
            Endpoint::read(&mut reader).map_err(|_| EndpointParsingError)?;

        // check if endpoint is valid
        if !Self::is_endpoint_valid(&endpoint) {
            return Err(EndpointParsingError);
        }
        Ok(endpoint)
    }

    fn named(
        name: &str,
        instance: EndpointInstance,
        type_: EndpointType,
    ) -> Result<Endpoint, InvalidEndpointError> {
        // make sure instance is valid
        if !Self::is_instance_valid(&instance) {
            return Err(InvalidEndpointError::InvalidInstance);
        }

        // convert name to bytes
        let name_bytes = Endpoint::name_to_bytes(name)?;

        Ok(Endpoint {
            type_,
            identifier: name_bytes,
            instance,
        })
    }

    fn name_to_bytes(name: &str) -> Result<[u8; 18], InvalidEndpointError> {
        let mut identifier = String::into_bytes(
            name.to_string().trim_end_matches('\0').to_string(),
        );
        // make sure length does not exceed 18 bytes
        if identifier.len() > 18 {
            return Err(InvalidEndpointError::MaxLengthExceeded);
        }
        // make sure length is at least 3 bytes
        if identifier.len() < 3 {
            return Err(InvalidEndpointError::MinLengthNotMet);
        }

        identifier.resize(18, 0);

        // make sure forbidden characters are not present
        if !Self::are_name_chars_valid(identifier.clone().try_into().unwrap()) {
            return Err(InvalidEndpointError::InvalidCharacters);
        };

        Ok(identifier.try_into().unwrap())
    }

    fn random_anonymous_id() -> [u8; 18] {
        let buffer = random::random_bytes_slice();
        if buffer.iter().any(|&b| b != 0) {
            return buffer;
        }
        // if all bytes are 0, we panic - this should not happen under normal circumstances
        panic!("Could not generate random anonymous id");
    }

    fn are_name_chars_valid(name: [u8; 18]) -> bool {
        let mut is_null = false;
        for c in name.iter() {
            // make sure '\0' bytes are only at the end if present
            if is_null && *c != 0x00 {
                return false;
            }
            if *c == 0x00 {
                is_null = true;
                continue;
            }
            // only allowed ranges 0-9, a-z, "_" and "-"
            if !(*c >= 0x30 && *c <= 0x39) && // 0-9
                !(*c >= 0x61 && *c <= 0x7A) && // a-z
                *c != 0x2D && // -
                *c != 0x5F
            {
                // _
                return false;
            }
        }
        true
    }

    fn is_endpoint_valid(endpoint: &Endpoint) -> bool {
        // make sure instance is valid
        if !Self::is_instance_valid(&endpoint.instance) {
            return false;
        }

        match endpoint.type_ {
            EndpointType::Person | EndpointType::Institution => {
                // name must be only contain valid characters
                Self::are_name_chars_valid(endpoint.identifier)
            }
            _ => true,
        }
    }

    fn is_instance_valid(endpoint_instance: &EndpointInstance) -> bool {
        match endpoint_instance {
            EndpointInstance::All => true,
            EndpointInstance::Any => true,
            EndpointInstance::Instance(instance) => {
                // instance must be between 1 and 65534
                *instance > 0 && *instance < 65535
            }
        }
    }

    pub fn to_binary(&self) -> [u8; 21] {
        let mut writer = Cursor::new(Vec::new());
        self.write(&mut writer).unwrap();
        writer.into_inner().try_into().unwrap()
    }

    // get endpoint type
    pub fn type_(&self) -> EndpointType {
        self.type_
    }

    // get endpoint instance
    pub fn instance(&self) -> EndpointInstance {
        self.instance
    }

    // check if endpoint is broadcast (instance is /*)
    pub fn is_broadcast(&self) -> bool {
        self.instance == EndpointInstance::All
    }

    // check if endpoint is local (@@local)
    pub fn is_local(&self) -> bool {
        self == &Endpoint::LOCAL
    }

    // check if endpoint is any (@@any)
    pub fn is_any(&self) -> bool {
        self == &Endpoint::ANY
    }

    // check if endpoint is an endpoint without a specific instance
    pub fn is_any_instance(&self) -> bool {
        self.instance == EndpointInstance::Any
    }

    // get the main endpoint (@person) of the endpoint without a specific instance
    pub fn any_instance_endpoint(&self) -> Endpoint {
        Endpoint {
            type_: self.type_,
            identifier: self.identifier,
            instance: EndpointInstance::Any,
        }
    }

    // get the broadcast endpoint (@person/*) of the endpoint
    pub fn broadcast(&self) -> Endpoint {
        Endpoint {
            type_: self.type_,
            identifier: self.identifier,
            instance: EndpointInstance::All,
        }
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.type_ {
            EndpointType::Anonymous => {
                // is @@any
                if self.identifier == [255; 18] {
                    write!(
                        f,
                        "{}{}",
                        Endpoint::PREFIX_ANONYMOUS,
                        Endpoint::ALIAS_ANY
                    )?;
                }
                // is @@local
                else if self.identifier == [0; 18] {
                    write!(
                        f,
                        "{}{}",
                        Endpoint::PREFIX_ANONYMOUS,
                        Endpoint::ALIAS_LOCAL
                    )?;
                }
                // is normal anonymous endpoint
                else {
                    write!(
                        f,
                        "{}{}",
                        Endpoint::PREFIX_ANONYMOUS,
                        buffer_to_hex(self.identifier.to_vec())
                    )?
                }
            }
            EndpointType::Person => write!(
                f,
                "{}{}",
                Endpoint::PREFIX_PERSON,
                str::from_utf8(&self.identifier)
                    .unwrap()
                    .trim_end_matches('\0')
            )?,
            EndpointType::Institution => write!(
                f,
                "{}{}",
                Endpoint::PREFIX_INSTITUTION,
                str::from_utf8(&self.identifier)
                    .unwrap()
                    .trim_end_matches('\0')
            )?,
        };

        match self.instance {
            EndpointInstance::Any => (),
            EndpointInstance::All => f.write_str("/*")?,
            EndpointInstance::Instance(instance) => write!(f, "/{instance}")?,
        };

        Ok(())
    }
}

impl FromStr for Endpoint {
    type Err = InvalidEndpointError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Endpoint::from_string(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utilities() {
        let endpoint: Endpoint = Endpoint::from_string("@ben/42").unwrap();
        assert!(!endpoint.is_any_instance());
        assert!(!endpoint.is_broadcast());

        let main_endpoint = endpoint.any_instance_endpoint();
        assert!(main_endpoint.is_any_instance());
        assert_eq!(main_endpoint.to_string(), "@ben");
        assert_eq!(main_endpoint.instance, EndpointInstance::Any);

        let broadcast_endpoint = endpoint.broadcast();
        assert!(broadcast_endpoint.is_broadcast());
        assert_eq!(broadcast_endpoint.to_string(), "@ben/*");
    }

    #[test]
    fn parse_from_string() {
        // valid personal endpoint
        let endpoint = Endpoint::from_string("@jonas").unwrap();
        assert_eq!(endpoint.type_, EndpointType::Person);
        assert_eq!(endpoint.instance, EndpointInstance::Any);
        assert_eq!(endpoint.to_string(), "@jonas");
        assert_eq!(
            endpoint.identifier,
            [106, 111, 110, 97, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );

        // valid institution endpoint
        let endpoint = Endpoint::from_string("@+unyt").unwrap();
        assert_eq!(endpoint.type_, EndpointType::Institution);
        assert_eq!(endpoint.instance, EndpointInstance::Any);
        assert_eq!(endpoint.to_string(), "@+unyt");

        // valid anonymous endpoint (@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA)
        let endpoint = Endpoint::from_string(
            &format!("@@{}", "A".repeat(18 * 2)).to_string(),
        )
        .unwrap();
        assert_eq!(endpoint.type_, EndpointType::Anonymous);
        assert_eq!(endpoint.instance, EndpointInstance::Any);
        assert_eq!(endpoint.to_string(), format!("@@{}", "A".repeat(18 * 2)));

        let valid_endpoint_names = vec![
            "@jonas",
            "@@any/*",
            "@@local",
            "@+unyt",
            "@test/42",
            "@test/*",
            "@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "@@BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
            "@+institution/42",
            "@+deno/9999",
            "@+deno/65534",
        ];
        for name in valid_endpoint_names {
            let endpoint = Endpoint::from_string(name).unwrap();
            assert_eq!(endpoint.to_string(), name);
        }
    }

    #[test]
    fn too_long() {
        let endpoint =
            Endpoint::person("too-long-endpoint-name", EndpointInstance::Any);
        assert_eq!(endpoint, Err(InvalidEndpointError::MaxLengthExceeded));

        let endpoint = Endpoint::from_string("@too-long-endpoint-name");
        assert_eq!(endpoint, Err(InvalidEndpointError::MaxLengthExceeded));

        let to_long_endpoint_names = vec![
            "@too-long-endpoint-name",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA",
            "@+too-long-endpoint-name",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA/0001",
        ];
        for name in to_long_endpoint_names {
            let endpoint = Endpoint::from_string(name);
            assert_eq!(endpoint, Err(InvalidEndpointError::MaxLengthExceeded));
        }
    }

    #[test]
    fn too_short() {
        let endpoint = Endpoint::person("ab", EndpointInstance::Any);
        assert_eq!(endpoint, Err(InvalidEndpointError::MinLengthNotMet));

        let endpoint =
            Endpoint::person("ab\0\0\0\0\0\0\0\0", EndpointInstance::Any);
        assert_eq!(endpoint, Err(InvalidEndpointError::MinLengthNotMet));

        let endpoint = Endpoint::from_string("@ab");
        assert_eq!(endpoint, Err(InvalidEndpointError::MinLengthNotMet));

        let to_short_endpoint_names = vec![
            "@ab",
            "@@ff",
            "@+ff",
            "@@fffffff",
            "@ab\0\0\0\0\0\0\0\0",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA/0001",
        ];
        for name in to_short_endpoint_names {
            let endpoint = Endpoint::from_string(name);
            assert_eq!(endpoint, Err(InvalidEndpointError::MinLengthNotMet));
        }
    }

    #[test]
    fn invalid_characters() {
        let endpoint = Endpoint::person("äüö", EndpointInstance::Any);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::person("__O", EndpointInstance::Any);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::person("#@!", EndpointInstance::Any);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::person("\0__", EndpointInstance::Any);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::from_string("@äüö");
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::from_string("@Jonas");
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint =
            Endpoint::from_string(&format!("@@{}X", "F".repeat(18 * 2 - 1)));
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let invalid_endpoint_names = vec![
            "@äüö",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFX",
            "@+äüö",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFX/0001",
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFX/0001",
            "test",
            "@dff@",
            "1",
            "",
        ];
        for name in invalid_endpoint_names {
            let endpoint = Endpoint::from_string(name);
            assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));
        }
    }

    #[test]
    fn invalid_instance() {
        let endpoint = Endpoint::person("test", EndpointInstance::Instance(0));
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidInstance));

        let endpoint =
            Endpoint::person("test", EndpointInstance::Instance(65535));
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidInstance));
    }

    #[test]
    fn special_instances() {
        let endpoint = Endpoint::from_string("@+unyt/0");
        assert_eq!(
            endpoint,
            Ok(Endpoint {
                type_: EndpointType::Institution,
                identifier: Endpoint::name_to_bytes("unyt").unwrap(),
                instance: EndpointInstance::Any,
            })
        );

        let endpoint = Endpoint::from_string("@+unyt/65535");
        assert_eq!(
            endpoint,
            Ok(Endpoint {
                type_: EndpointType::Institution,
                identifier: Endpoint::name_to_bytes("unyt").unwrap(),
                instance: EndpointInstance::All,
            })
        );

        let endpoint = Endpoint::from_string("@+unyt/*");
        assert_eq!(
            endpoint,
            Ok(Endpoint {
                type_: EndpointType::Institution,
                identifier: Endpoint::name_to_bytes("unyt").unwrap(),
                instance: EndpointInstance::All,
            })
        );
    }

    #[test]
    fn any_instance() {
        // @@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF/0
        let binary = [
            EndpointType::Anonymous as u8,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            0x00,
            0x00,
        ];
        let endpoint = Endpoint::from_binary(binary);
        assert_eq!(endpoint, Ok(Endpoint::ANY));
    }

    #[test]
    fn special_endpoints() {
        let endpoint = Endpoint::from_string("@@any").unwrap();
        assert_eq!(endpoint.to_string(), "@@any");
        assert_eq!(endpoint, Endpoint::ANY);

        let endpoint = Endpoint::from_string("@@any/42").unwrap();
        assert_eq!(endpoint.to_string(), "@@any/42");

        let endpoint = Endpoint::from_string("@@local").unwrap();
        assert_eq!(endpoint.to_string(), "@@local");
        assert_eq!(endpoint, Endpoint::LOCAL);

        let endpoint =
            Endpoint::from_string("@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
                .unwrap();
        assert_eq!(endpoint, Endpoint::ANY);

        let endpoint =
            Endpoint::from_string("@@000000000000000000000000000000000000")
                .unwrap();
        assert_eq!(endpoint, Endpoint::LOCAL);
    }

    #[test]
    fn format_named_endpoint() {
        let endpoint = Endpoint::person("test", EndpointInstance::Any).unwrap();
        assert_eq!(endpoint.to_string(), "@test");

        let endpoint =
            Endpoint::institution("test", EndpointInstance::Any).unwrap();
        assert_eq!(endpoint.to_string(), "@+test");

        let endpoint =
            Endpoint::person("test", EndpointInstance::Instance(42)).unwrap();
        assert_eq!(endpoint.to_string(), "@test/42");

        let endpoint = Endpoint::person("test", EndpointInstance::All).unwrap();
        assert_eq!(endpoint.to_string(), "@test/*");
    }

    #[test]
    fn format_anonymous_endpoint() {
        let endpoint =
            Endpoint::anonymous([0xaa; 18], EndpointInstance::Any).unwrap();
        assert_eq!(
            endpoint.to_string(),
            "@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );

        let endpoint =
            Endpoint::from_string("@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/42")
                .unwrap();
        assert_eq!(
            endpoint.to_string(),
            "@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/42"
        );
    }
}

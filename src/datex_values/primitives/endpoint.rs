use crate::global::protocol_structures::addressing::EndpointType;
use crate::utils::buffers::buffer_to_hex;
use binrw::{BinRead, BinWrite};
use hex::decode;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::io::Cursor;

#[derive(BinWrite, BinRead, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum EndpointInstance {
    #[br(magic = 0u16)]
    Main,
    #[br(magic = 65535u16)]
    Any,
    Instance(u16),
}

#[derive(BinWrite, BinRead, Debug, Clone, Hash, PartialEq, Eq)]
#[brw(little)]
pub struct Endpoint {
    // 1 byte type, 18 bytes name, 2 bytes instance
    type_: EndpointType,
    identifier: [u8; 18],
    instance: EndpointInstance,
}

#[derive(PartialEq, Debug)]
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
    pub const PREFIX_PERSON: &'static str = "@";
    pub const PREFIX_INSTITUTION: &'static str = "@+";
    pub const PREFIX_ANONYMOUS: &'static str = "@@";

    pub const ALIAS_LOCAL: &'static str = "local";
    pub const ALIAS_ANY: &'static str = "any";

    pub const ANY: Endpoint = Endpoint {
        type_: EndpointType::Any,
        identifier: [255; 18],
        instance: EndpointInstance::Any,
    };
    pub const LOCAL: Endpoint = Endpoint {
        type_: EndpointType::Local,
        identifier: [0; 18],
        instance: EndpointInstance::Main,
    };

    // create default id endpoint (@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
    pub fn new_anonymous(
        identifier: [u8; 18],
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointError> {
        if identifier == [0; 18] || identifier == [255; 18] {
            return Err(InvalidEndpointError::ReservedName);
        }
        //  {
        //     if instance == EndpointInstance::Main {
        //         return Ok(Endpoint::LOCAL);
        //     } else {
        //         return Err(InvalidEndpointNameError::InvalidInstance);
        //     }
        // } else if identifier == [255; 18] {
        //     if instance == EndpointInstance::Any {
        //         return Ok(Endpoint::ANY);
        //     }
        // TODO: shall we allow instance for @@any?
        // } else {
        //     return Err(InvalidEndpointNameError::InvalidInstance);
        // }
        Ok(Endpoint {
            type_: EndpointType::Anonymous,
            identifier,
            instance,
        })
    }

    // create alias endpoint (@person)
    pub fn new_person(
        name: &str,
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointError> {
        Self::new_named(name, instance, EndpointType::Person)
    }

    // create institution endpoint (@+institution)
    pub fn new_institution(
        name: &str,
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointError> {
        Self::new_named(name, instance, EndpointType::Institution)
    }

    // create endpoint from string (@person/42, @@local, @+unyt)
    pub fn new_from_string(
        name: &str,
    ) -> Result<Endpoint, InvalidEndpointError> {
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
        let mut instance = EndpointInstance::Main;
        // check if instance is present
        if name.contains('/') {
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() != 2 {
                return Err(InvalidEndpointError::InvalidCharacters);
            }
            name_part = parts[0].to_string();
            let instance_str = parts[1];
            if instance_str == "*" {
                instance = EndpointInstance::Any;
            } else {
                let instance_num = instance_str
                    .parse::<u16>()
                    .map_err(|_| InvalidEndpointError::InvalidInstance)?;
                instance = EndpointInstance::Instance(instance_num);
            }
        }

        let endpoint = match name_part {
            // TODO shall we allow instance for @@any?
            s if s.starts_with(&format!(
                "{}{}",
                Endpoint::PREFIX_ANONYMOUS,
                Endpoint::ALIAS_ANY
            )) =>
            {
                Ok(Endpoint {
                    type_: EndpointType::Any,
                    identifier: [255u8; 18],
                    instance,
                })
            }
            // TODO shall we allow instance for @@local?
            s if s.starts_with(&format!(
                "{}{}",
                Endpoint::PREFIX_ANONYMOUS,
                Endpoint::ALIAS_LOCAL
            )) =>
            {
                Ok(Endpoint {
                    type_: EndpointType::Local,
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
                let bytes = decode(&s)
                    .map_err(|_| InvalidEndpointError::InvalidCharacters)?;
                let byte_slice: &[u8] = &bytes;
                Endpoint::new_anonymous(
                    byte_slice.try_into().unwrap(),
                    instance,
                )
            }
            s if s.starts_with(Endpoint::PREFIX_INSTITUTION) => {
                Endpoint::new_named(
                    &s[2..],
                    instance,
                    EndpointType::Institution,
                )
            }
            s if s.starts_with(Endpoint::PREFIX_PERSON) => {
                Endpoint::new_named(&s[1..], instance, EndpointType::Person)
            }
            _ => return Err(InvalidEndpointError::InvalidCharacters),
        };
        endpoint
    }

    // parse endpoint from binary
    pub fn new_from_binary(
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

    fn new_named(
        name: &str,
        instance: EndpointInstance,
        type_: EndpointType,
    ) -> Result<Endpoint, InvalidEndpointError> {
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
        // make sure instance is valid
        if !Self::is_instance_valid(&instance) {
            return Err(InvalidEndpointError::InvalidInstance);
        }

        identifier.resize(18, 0);

        // make sure forbidden characters are not present
        if !Self::are_name_chars_valid(identifier.clone().try_into().unwrap()) {
            return Err(InvalidEndpointError::InvalidCharacters);
        }

        Ok(Endpoint {
            type_,
            identifier: identifier.try_into().unwrap(),
            instance,
        })
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
            // only allowed ranges 0-9, A-Z, a-z, "_" and "-"
            if !(*c >= 0x30 && *c <= 0x39) && // 0-9
                !(*c >= 0x41 && *c <= 0x5A) && // A-Z
                !(*c >= 0x61 && *c <= 0x7A) && // a-z
                *c != 0x2D && // -
                *c != 0x5F
            {
                // _
                return false;
            }
            // forbidden characters: O, I
            if *c == 0x4F || *c == 0x49 {
                return false;
            }
        }
        true
    }

    fn is_instance_valid(endpoint_instance: &EndpointInstance) -> bool {
        match endpoint_instance {
            EndpointInstance::Main => true,
            EndpointInstance::Any => true,
            EndpointInstance::Instance(instance) => {
                // instance must be between 1 and 65534
                *instance > 0 && *instance < 65535
            }
        }
    }

    fn is_endpoint_valid(endpoint: &Endpoint) -> bool {
        // make sure instance is valid
        if !Self::is_instance_valid(&endpoint.instance) {
            return false;
        }
        match endpoint.type_ {
            EndpointType::Any => {
                // instance must be Any
                endpoint.instance == EndpointInstance::Any &&
                    // identifier must be all 1
                    endpoint.identifier == [255u8; 18]
            }
            EndpointType::Person | EndpointType::Institution => {
                // name must be only contain valid characters
                Self::are_name_chars_valid(endpoint.identifier)
            }
            _ => true,
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
        self.instance == EndpointInstance::Any
    }

    // check if endpoint is local (@@local)
    pub fn is_local(&self) -> bool {
        self == &Endpoint::LOCAL
    }

    // check if endpoint is any (@@any)
    pub fn is_any(&self) -> bool {
        self == &Endpoint::ANY
    }

    // check if endpoint is main (@person) without instance
    pub fn is_main(&self) -> bool {
        self.instance == EndpointInstance::Main
    }

    // get the main endpoint (@person) of the endpoint without instance
    pub fn main(&self) -> Endpoint {
        Endpoint {
            type_: self.type_,
            identifier: self.identifier,
            instance: EndpointInstance::Main,
        }
    }

    // get the broadcast endpoint (@person/*) of the endpoint
    pub fn broadcast(&self) -> Endpoint {
        Endpoint {
            type_: self.type_,
            identifier: self.identifier,
            instance: EndpointInstance::Any,
        }
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.type_ {
            EndpointType::Anonymous => write!(
                f,
                "{}{}",
                Endpoint::PREFIX_ANONYMOUS,
                buffer_to_hex(self.identifier.to_vec())
            )?,
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
            EndpointType::Local => f.write_str(
                format!("{}local", Endpoint::PREFIX_ANONYMOUS).as_str(),
            )?,
            EndpointType::Any => f.write_str(
                format!("{}any", Endpoint::PREFIX_ANONYMOUS).as_str(),
            )?,
        };

        match self.instance {
            EndpointInstance::Main => (),
            EndpointInstance::Any => f.write_str("/*")?,
            EndpointInstance::Instance(instance) => write!(f, "/{}", instance)?,
        };

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn utilities() {
        let endpoint: Endpoint = Endpoint::new_from_string("@ben/42").unwrap();
        assert!(!endpoint.is_main());
        assert!(!endpoint.is_broadcast());

        let main_endpoint = endpoint.main();
        assert!(main_endpoint.is_main());
        assert_eq!(main_endpoint.to_string(), "@ben");
        assert_eq!(main_endpoint.instance, EndpointInstance::Main);

        let broadcast_endpoint = endpoint.broadcast();
        assert!(broadcast_endpoint.is_broadcast());
        assert_eq!(broadcast_endpoint.to_string(), "@ben/*");
    }

    #[test]
    fn parse_from_string() {
        // valid personal endpoint
        let endpoint = Endpoint::new_from_string("@jonas").unwrap();
        assert!(endpoint.type_ == EndpointType::Person);
        assert!(endpoint.instance == EndpointInstance::Main);
        assert_eq!(endpoint.to_string(), "@jonas");
        assert_eq!(
            endpoint.identifier,
            [106, 111, 110, 97, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );

        // valid institution endpoint
        let endpoint = Endpoint::new_from_string("@+unyt").unwrap();
        assert!(endpoint.type_ == EndpointType::Institution);
        assert!(endpoint.instance == EndpointInstance::Main);
        assert_eq!(endpoint.to_string(), "@+unyt");

        // valid anonymous endpoint (@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA)
        let endpoint = Endpoint::new_from_string(
            &format!("@@{}", "A".repeat(18 * 2)).to_string(),
        )
        .unwrap();
        assert!(endpoint.type_ == EndpointType::Anonymous);
        assert!(endpoint.instance == EndpointInstance::Main);
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
            let endpoint = Endpoint::new_from_string(name).unwrap();
            assert_eq!(endpoint.to_string(), name);
        }
    }

    #[test]
    fn too_long() {
        let endpoint = Endpoint::new_person(
            "too-long-endpoint-name",
            EndpointInstance::Main,
        );
        assert_eq!(endpoint, Err(InvalidEndpointError::MaxLengthExceeded));

        let endpoint = Endpoint::new_from_string("@too-long-endpoint-name");
        assert_eq!(endpoint, Err(InvalidEndpointError::MaxLengthExceeded));

        let to_long_endpoint_names = vec![
            "@too-long-endpoint-name",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA",
            "@+too-long-endpoint-name",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA/0001",
        ];
        for name in to_long_endpoint_names {
            let endpoint = Endpoint::new_from_string(name);
            assert_eq!(endpoint, Err(InvalidEndpointError::MaxLengthExceeded));
        }
    }

    #[test]
    fn too_short() {
        let endpoint = Endpoint::new_person("ab", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointError::MinLengthNotMet));

        let endpoint =
            Endpoint::new_person("ab\0\0\0\0\0\0\0\0", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointError::MinLengthNotMet));

        let endpoint = Endpoint::new_from_string("@ab");
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
            let endpoint = Endpoint::new_from_string(name);
            assert_eq!(endpoint, Err(InvalidEndpointError::MinLengthNotMet));
        }
    }

    #[test]
    fn invalid_characters() {
        let endpoint = Endpoint::new_person("äüö", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::new_person("__O", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::new_person("#@!", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::new_person("\0__", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::new_from_string("@äüö");
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));

        let endpoint = Endpoint::new_from_string(&format!(
            "@@{}X",
            "F".repeat(18 * 2 - 1)
        ));
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
            let endpoint = Endpoint::new_from_string(name);
            assert_eq!(endpoint, Err(InvalidEndpointError::InvalidCharacters));
        }
    }

    #[test]
    fn invalid_instance() {
        let endpoint =
            Endpoint::new_person("test", EndpointInstance::Instance(0));
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidInstance));

        let endpoint =
            Endpoint::new_person("test", EndpointInstance::Instance(65535));
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidInstance));

        let endpoint = Endpoint::new_from_string("@unyt/0");
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidInstance));
        let endpoint = Endpoint::new_from_string("@+unyt/0");
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidInstance));
        let endpoint = Endpoint::new_from_string("@+unyt/65535");
        assert_eq!(endpoint, Err(InvalidEndpointError::InvalidInstance));
    }

    #[test]
    fn any_instance() {
        let binary = [0xFF; 21];
        let endpoint = Endpoint::new_from_binary(binary);
        assert_eq!(endpoint, Ok(Endpoint::ANY));
    }

    #[test]
    fn special_endpoints() {
        let endpoint = Endpoint::new_from_string("@@any").unwrap();
        assert_eq!(endpoint.to_string(), "@@any/*");
        assert_eq!(endpoint, Endpoint::ANY);

        let endpoint = Endpoint::new_from_string("@@any/42").unwrap();
        assert_eq!(endpoint.to_string(), "@@any/42");

        let endpoint = Endpoint::new_from_string("@@local").unwrap();
        assert_eq!(endpoint.to_string(), "@@local");
        assert_eq!(endpoint, Endpoint::LOCAL);

        let endpoint =
            Endpoint::new_from_string("@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        assert_eq!(endpoint, Err(InvalidEndpointError::ReservedName));

        let endpoint =
            Endpoint::new_from_string("@@000000000000000000000000000000000000");
        assert_eq!(endpoint, Err(InvalidEndpointError::ReservedName));
    }

    #[test]
    fn format_named_endpoint() {
        let endpoint =
            Endpoint::new_person("test", EndpointInstance::Main).unwrap();
        assert_eq!(endpoint.to_string(), "@test");

        let endpoint =
            Endpoint::new_institution("test", EndpointInstance::Main).unwrap();
        assert_eq!(endpoint.to_string(), "@+test");

        let endpoint =
            Endpoint::new_person("test", EndpointInstance::Instance(42))
                .unwrap();
        assert_eq!(endpoint.to_string(), "@test/42");

        let endpoint =
            Endpoint::new_person("test", EndpointInstance::Any).unwrap();
        assert_eq!(endpoint.to_string(), "@test/*");
    }

    #[test]
    fn format_anonymous_endpoint() {
        let endpoint =
            Endpoint::new_anonymous([0xaa; 18], EndpointInstance::Main)
                .unwrap();
        assert_eq!(
            endpoint.to_string(),
            "@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );

        let endpoint = Endpoint::new_from_string(
            "@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/42",
        )
        .unwrap();
        assert_eq!(
            endpoint.to_string(),
            "@@AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/42"
        );
    }
}

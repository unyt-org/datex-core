use crate::global::protocol_structures::addressing::EndpointType;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::utils::buffers::buffer_to_hex;
use crate::utils::{
    buffers::{self, append_u16, append_u8, read_u16, read_u8, read_vec_slice},
    color::Color,
};
use binrw::{endian, BinRead, BinWrite};
use hex::{decode, decode_to_slice};
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
pub enum InvalidEndpointNameError {
    InvalidCharacters,
    MaxLengthExceeded,
    MinLengthNotMet,
    InvalidInstance,
}
#[derive(PartialEq, Debug)]
pub struct InvalidEndpointError;

impl Endpoint {
    pub const PREFIX_PERSON: &'static str = "@";
    pub const PREFIX_INSTITUTION: &'static str = "@+";
    pub const PREFIX_ANONYMOUS: &'static str = "@@";

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

    // create default id endpoint (@@1234567890, @@local)
    pub fn new_anonymous(
        identifier: [u8; 18],
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointNameError> {
        if identifier == [0; 18] {
            if instance == EndpointInstance::Main {
                return Ok(Endpoint::LOCAL);
            } else {
                return Err(InvalidEndpointNameError::InvalidInstance);
            }
        } else if identifier == [255; 18] {
            if instance == EndpointInstance::Any {
                return Ok(Endpoint::ANY);
            }
            // TODO: shall we allow instance for @@any?
            // } else {
            //     return Err(InvalidEndpointNameError::InvalidInstance);
            // }
        }
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
    ) -> Result<Endpoint, InvalidEndpointNameError> {
        Self::new_named(name, instance, EndpointType::Person)
    }

    // create institution endpoint (@+institution)
    pub fn new_institution(
        name: &str,
        instance: EndpointInstance,
    ) -> Result<Endpoint, InvalidEndpointNameError> {
        Self::new_named(name, instance, EndpointType::Institution)
    }

    pub fn new_from_string(
        name: &str,
    ) -> Result<Endpoint, InvalidEndpointNameError> {
        let name = name.to_string();
        if name == "@@any" {
            return Ok(Endpoint::ANY);
        } else if name == "@@local" {
            return Ok(Endpoint::LOCAL);
        }

        let mut name_part = name.clone();
        let mut instance = EndpointInstance::Main;
        // check if instance is present
        if name.contains('/') {
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() != 2 {
                return Err(InvalidEndpointNameError::InvalidCharacters);
            }
            name_part = parts[0].to_string();
            let instance_str = parts[1];
            if instance_str == "*" {
                instance = EndpointInstance::Any;
            } else {
                let instance_num = instance_str
                    .parse::<u16>()
                    .map_err(|_| InvalidEndpointNameError::InvalidInstance)?;
                instance = EndpointInstance::Instance(instance_num);
            }
        }

        let endpoint = match name_part {
            s if s.starts_with("@@any") => {
                Endpoint::new_anonymous([255u8; 18], instance)
            }
            s if s.starts_with("@@local") => {
                Endpoint::new_anonymous([0u8; 18], instance)
            }
            s if s.starts_with("@@") => {
                let s = s.trim_start_matches("@@");
                if s.len() < 18 * 2 {
                    return Err(InvalidEndpointNameError::MinLengthNotMet);
                } else if s.len() > 18 * 2 {
                    return Err(InvalidEndpointNameError::MaxLengthExceeded);
                }
                let bytes = decode(&s)
                    .map_err(|_| InvalidEndpointNameError::InvalidCharacters)?;
                let byte_slice: &[u8] = &bytes;
                Endpoint::new_anonymous(
                    byte_slice.try_into().unwrap(),
                    instance,
                )
            }
            s if s.starts_with("@+") => Endpoint::new_named(
                &s[2..],
                instance,
                EndpointType::Institution,
            ),
            s if s.starts_with("@") => {
                Endpoint::new_named(&s[1..], instance, EndpointType::Person)
            }
            _ => return Err(InvalidEndpointNameError::InvalidCharacters),
        };
        endpoint
    }

    pub fn new_from_binary(
        binary: [u8; 21],
    ) -> Result<Endpoint, InvalidEndpointError> {
        let mut reader = Cursor::new(binary);
        let endpoint =
            Endpoint::read(&mut reader).map_err(|_| InvalidEndpointError)?;

        // check if endpoint is valid
        if !Self::is_endpoint_valid(&endpoint) {
            return Err(InvalidEndpointError);
        }
        Ok(endpoint)
    }

    fn new_named(
        name: &str,
        instance: EndpointInstance,
        type_: EndpointType,
    ) -> Result<Endpoint, InvalidEndpointNameError> {
        let mut identifier = String::into_bytes(
            name.to_string().trim_end_matches('\0').to_string(),
        );
        // make sure length does not exceed 18 bytes
        if identifier.len() > 18 {
            return Err(InvalidEndpointNameError::MaxLengthExceeded);
        }
        // make sure length is at least 3 bytes
        if identifier.len() < 3 {
            return Err(InvalidEndpointNameError::MinLengthNotMet);
        }
        // make sure instance is valid
        if !Self::is_instance_valid(&instance) {
            return Err(InvalidEndpointNameError::InvalidInstance);
        }

        identifier.resize(18, 0);

        // make sure forbidden characters are not present
        if !Self::are_name_chars_valid(identifier.clone().try_into().unwrap()) {
            return Err(InvalidEndpointNameError::InvalidCharacters);
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

    pub fn type_(&self) -> EndpointType {
        self.type_
    }

    pub fn instance(&self) -> EndpointInstance {
        self.instance
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
        assert_eq!(endpoint, Err(InvalidEndpointNameError::MaxLengthExceeded));

        let endpoint = Endpoint::new_from_string("@too-long-endpoint-name");
        assert_eq!(endpoint, Err(InvalidEndpointNameError::MaxLengthExceeded));

        let to_long_endpoint_names = vec![
            "@too-long-endpoint-name",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA",
            "@+too-long-endpoint-name",
            "@@FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA/0001",
        ];
        for name in to_long_endpoint_names {
            let endpoint = Endpoint::new_from_string(name);
            assert_eq!(
                endpoint,
                Err(InvalidEndpointNameError::MaxLengthExceeded)
            );
        }
    }

    #[test]
    fn too_short() {
        let endpoint = Endpoint::new_person("ab", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointNameError::MinLengthNotMet));

        let endpoint =
            Endpoint::new_person("ab\0\0\0\0\0\0\0\0", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointNameError::MinLengthNotMet));

        let endpoint = Endpoint::new_from_string("@ab");
        assert_eq!(endpoint, Err(InvalidEndpointNameError::MinLengthNotMet));

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
            assert_eq!(
                endpoint,
                Err(InvalidEndpointNameError::MinLengthNotMet)
            );
        }
    }

    #[test]
    fn invalid_characters() {
        let endpoint = Endpoint::new_person("äüö", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidCharacters));

        let endpoint = Endpoint::new_person("__O", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidCharacters));

        let endpoint = Endpoint::new_person("#@!", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidCharacters));

        let endpoint = Endpoint::new_person("\0__", EndpointInstance::Main);
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidCharacters));

        let endpoint = Endpoint::new_from_string("@äüö");
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidCharacters));

        let endpoint = Endpoint::new_from_string(&format!(
            "@@{}X",
            "F".repeat(18 * 2 - 1)
        ));
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidCharacters));

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
            assert_eq!(
                endpoint,
                Err(InvalidEndpointNameError::InvalidCharacters)
            );
        }
    }

    #[test]
    fn invalid_instance() {
        let endpoint =
            Endpoint::new_person("test", EndpointInstance::Instance(0));
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidInstance));

        let endpoint =
            Endpoint::new_person("test", EndpointInstance::Instance(65535));
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidInstance));

        let endpoint = Endpoint::new_from_string("@unyt/0");
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidInstance));
        let endpoint = Endpoint::new_from_string("@+unyt/0");
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidInstance));
        let endpoint = Endpoint::new_from_string("@+unyt/65535");
        assert_eq!(endpoint, Err(InvalidEndpointNameError::InvalidInstance));
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

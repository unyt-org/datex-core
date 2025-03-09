use regex::Regex;
use std::hash::{Hash, Hasher};

use crate::utils::{
    buffers::{self, append_u16, append_u8, read_slice, read_u16, read_u8},
    color::Color,
};

#[derive(Debug, Clone, PartialEq)]
pub enum EndpointType {
    Id,
    PersonAlias,
    InstitutionAlias,
}

#[derive(Debug, Clone)]
pub struct Endpoint {
    name: String,
    endpoint_type: EndpointType,
    instance: u16,
    binary: Vec<u8>, // 1 byte type, 18 bytes name, 2 bytes instance
}

impl Hash for Endpoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.binary.as_slice());
    }
}

impl PartialEq for Endpoint {
    fn eq(&self, other: &Self) -> bool {
        self.binary == other.binary
    }
}

impl Eq for Endpoint {}

impl Endpoint {
    pub const ANY_INSTANCE: u16 = 0;

    // create default id endpoint (@@1234567890, @@local)
    pub fn new(name_binary: &Vec<u8>, instance: u16) -> Endpoint {
        let mut name = buffers::buffer_to_hex(name_binary.to_vec());
        name = Regex::new(r"(00)*$")
            .unwrap()
            .replace_all(&name, "")
            .to_string();
        if name == "" {
            name = "local".to_string()
        } else if name == "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF" {
            name = "any".to_string()
        }

        Endpoint {
            name,
            endpoint_type: EndpointType::Id,
            instance,
            binary: Self::to_binary(EndpointType::Id, name_binary, instance),
        }
    }

    // create alias endpoint (@person)
    pub fn new_person(name: &str, instance: u16) -> Endpoint {
        Endpoint {
            name: name.trim_matches(char::from(0)).to_string(),
            endpoint_type: EndpointType::PersonAlias,
            instance,
            binary: Self::to_binary(
                EndpointType::PersonAlias,
                &Self::encode_name_binary(name.to_string()),
                instance,
            ),
        }
    }

    // create institution endpoint (@+institution)
    pub fn new_institution(name: &str, instance: u16) -> Endpoint {
        Endpoint {
            name: name.trim_matches(char::from(0)).to_string(),
            endpoint_type: EndpointType::InstitutionAlias,
            instance,
            binary: Self::to_binary(
                EndpointType::InstitutionAlias,
                &Self::encode_name_binary(name.to_string()),
                instance,
            ),
        }
    }

    pub fn new_from_binary(binary: &Vec<u8>) -> Endpoint {
        let index = &mut 0;
        let endpoint_type_bin = read_u8(binary, index);
        let endpoint_type = match endpoint_type_bin {
            2 => EndpointType::InstitutionAlias,
            1 => EndpointType::PersonAlias,
            _ => EndpointType::Id,
        };

        let name = &read_slice(binary, index, 18);
        let instance = read_u16(binary, index);

        match endpoint_type {
            EndpointType::InstitutionAlias => {
                Self::new_institution(&Self::decode_name_binary(name), instance)
            }
            EndpointType::PersonAlias => {
                Self::new_person(&Self::decode_name_binary(name), instance)
            }
            EndpointType::Id => Self::new(name, instance),
        }
    }

    // convert string name to binary representation
    fn encode_name_binary(name: String) -> Vec<u8> {
        if name.len() > 18 {
            panic!("Endpoint name exceeds maximum of 18 bytes");
        }
        // TODO: maybe 6 bit charset?
        return String::into_bytes(name);
    }

    // convert binary representation with null terminators to string
    fn decode_name_binary(name_binary: &Vec<u8>) -> String {
        if name_binary.len() > 18 {
            panic!("Endpoint name exceeds maximum of 18 bytes");
        }

        let name_utf8 = String::from_utf8(name_binary.to_vec())
            .expect("could not read endpoint name");
        // remove \0
        return name_utf8.trim_matches(char::from(0)).to_string();
    }

    // get the binary representation for an endpoint
    // 1 byte type, 18 bytes name, 2 bytes instance
    fn to_binary(
        endpoint_type: EndpointType,
        name_binary: &Vec<u8>,
        instance: u16,
    ) -> Vec<u8> {
        if name_binary.len() > 18 {
            // might include null terminator
            panic!("Endpoint name exceeds maximum of 18 bytes");
        }
        let name_sized = &mut name_binary.to_vec();
        name_sized.resize(18, 0);
        let binary = &mut Vec::<u8>::with_capacity(21);

        append_u8(binary, endpoint_type as u8);
        binary.extend_from_slice(name_sized);
        append_u16(binary, instance);

        return binary.to_vec();
    }

    pub fn get_binary(&self) -> &Vec<u8> {
        return &self.binary;
    }

    pub fn get_type(&self) -> &EndpointType {
        return &self.endpoint_type;
    }

    pub fn get_instance(&self) -> u16 {
        return self.instance;
    }

    pub fn to_string(&self, colorized: bool) -> String {
        let mut main = match self.endpoint_type {
            EndpointType::Id => format!(
                "{}@@{}",
                (if colorized {
                    Color::ENDPOINT.as_ansi_rgb()
                } else {
                    "".to_string()
                }),
                self.name
            ),
            EndpointType::PersonAlias => format!(
                "{}@{}",
                (if colorized {
                    Color::EndpointPerson.as_ansi_rgb()
                } else {
                    "".to_string()
                }),
                self.name
            ),
            EndpointType::InstitutionAlias => format!(
                "{}@+{}",
                (if colorized {
                    Color::EndpointInstitution.as_ansi_rgb()
                } else {
                    "".to_string()
                }),
                self.name
            ),
        };
        // if self.subspaces.is_some() {
        // 	for subspace in self.subspaces.as_ref().unwrap() {
        // 		if colorized {main += &Color::DEFAULT.as_ansi_rgb()}
        // 		main += ".";
        // 		if colorized {main += &Color::DefaultLight.as_ansi_rgb()}
        // 		main += &subspace;
        // 	}
        // }

        if self.instance != Endpoint::ANY_INSTANCE {
            main += &format!("/{:04X}", self.instance);
        }

        return main;
    }
}

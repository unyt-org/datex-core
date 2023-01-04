use crate::{utils::{buffers::{self, append_u8, append_u16, read_u8, read_string_utf8, read_slice, read_u16, buffer_to_hex, buffer_to_hex_advanced}, color::Color}};

#[derive(Debug, Clone, PartialEq)]
pub enum EndpointType {
	Id,
	PersonAlias,
	InstitutionAlias
}


#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
	name: String,
	endpoint_type: EndpointType,
	subspaces: Option<Vec<String>>,
	instance: u16,
	binary: Vec<u8> // 1 byte type, 18 bytes name, 2 bytes instance
}

impl Endpoint {
	
	pub const ANY_INSTANCE:u16 = 0;

	// create default id endpoint (@@1234567890)
	pub fn new(name_binary:&Vec<u8>, instance: u16, subspaces:Option<Vec<String>>) -> Endpoint {
		let name = buffers::buffer_to_hex(name_binary.to_vec());
		Endpoint {
			name,
			endpoint_type: EndpointType::Id,
			instance,
			subspaces,
			binary: Self::to_binary(EndpointType::Id, name_binary, instance)
		}
	}

	// create alias endpoint (@person)
	pub fn new_person_alias(name:&str, instance: u16, subspaces:Option<Vec<String>>) -> Endpoint {
		Endpoint {
			name: name.to_string(),
			endpoint_type: EndpointType::PersonAlias,
			instance,
			subspaces,
			binary: Self::to_binary(EndpointType::PersonAlias, &Self::encode_name_binary(name.to_string()), instance)
		}
	}

	// create institution endpoint (@+institution)
	pub fn new_institution_alias(name:&str, instance: u16, subspaces:Option<Vec<String>>) -> Endpoint {
		Endpoint {
			name: name.to_string(),
			endpoint_type: EndpointType::InstitutionAlias,
			instance,
			subspaces,
			binary: Self::to_binary(EndpointType::InstitutionAlias, &Self::encode_name_binary(name.to_string()), instance)
		}
	}

	pub fn new_from_binary(binary:&Vec<u8>) -> Endpoint {
		let index = &mut 0;
		let endpoint_type_bin = read_u8(binary, index);
		let endpoint_type = match endpoint_type_bin {
			2 => EndpointType::InstitutionAlias,
			1 => EndpointType::PersonAlias,
			_ => EndpointType::Id
		};

		let name = &read_slice(binary, index, 18);
		let instance = read_u16(binary, index);

		match endpoint_type {
			EndpointType::InstitutionAlias => Self::new_institution_alias(&Self::decode_name_binary(name), instance, None),
			EndpointType::PersonAlias => Self::new_person_alias(&Self::decode_name_binary(name), instance, None),
			EndpointType::Id => Self::new(name, instance, None)
		}

	}


	// convert string name to binary representation
	fn encode_name_binary(name: String) -> Vec<u8> {
		if name.len()>18 {
			panic!("Endpoint name exceeds maximum of 18 bytes");
		}
		// TODO: maybe 6 bit charset?
		return String::into_bytes(name);
	}

	// convert binary representation with null terminators to string
	fn decode_name_binary(name_binary: &Vec<u8>) -> String {
		if name_binary.len()>18 {
			panic!("Endpoint name exceeds maximum of 18 bytes");
		}

		let name_utf8 = String::from_utf8(name_binary.to_vec()).expect("could not read endpoint name");
		// remove \0
		return name_utf8.trim_matches(char::from(0)).to_string();
	}


	// get the binary representation for an endpoint
	// 1 byte type, 18 bytes name, 2 bytes instance
	fn to_binary(endpoint_type: EndpointType, name_binary:&Vec<u8>, instance: u16) -> Vec<u8> {
		if name_binary.len()>18 { // might include null terminator
			panic!("Endpoint name exceeds maximum of 18 bytes");
		}
		let name_sized = &mut name_binary.to_vec();
		name_sized.resize(18, 0);
		let binary = &mut Vec::<u8>::with_capacity(21);

		append_u8(binary, endpoint_type as u8);
		binary.extend_from_slice(name_sized);
		append_u16(binary, instance);

		return binary.to_vec()
	}


	pub fn get_binary(&self) -> &Vec<u8>{
		return &self.binary;
	}

	pub fn get_type(&self) -> &EndpointType {
		return &self.endpoint_type;
	}

	pub fn get_instance(&self) -> u16 {
		return self.instance;
	}

	pub fn to_string(&self, colorized:bool) -> String {
		let mut main = match self.endpoint_type {
			EndpointType::Id => format!("{}@@{}",				 	(if colorized {Color::ENDPOINT.as_ansi_rgb()} else {"".to_string()}), self.name),
			EndpointType::PersonAlias => format!("{}@{}", 			(if colorized {Color::EndpointPerson.as_ansi_rgb()} else {"".to_string()}), self.name),
			EndpointType::InstitutionAlias => format!("{}@+{}", 	(if colorized {Color::EndpointInstitution.as_ansi_rgb()} else {"".to_string()}), self.name)
		};
		if self.subspaces.is_some() {
			for subspace in self.subspaces.as_ref().unwrap() {
				if colorized {main += &Color::DEFAULT.as_ansi_rgb()}
				main += ".";
				if colorized {main += &Color::DefaultLight.as_ansi_rgb()}
				main += &subspace;
			}
		}

		if self.instance != Endpoint::ANY_INSTANCE {
			main += &format!("/{:X}", self.instance);
		}
		
		return main;
	}

}
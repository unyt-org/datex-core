use crate::{utils::{buffers, color::Color}};

#[derive(Clone)]
pub enum EndpointType {
	Id,
	PersonAlias,
	InstitutionAlias
}


#[derive(Clone)]
pub struct Endpoint {
	name: String,
	name_binary: Option<Vec<u8>>,
	endpoint_type: EndpointType,
	subspaces:Vec<String>
}

impl Endpoint {
	
	// create default id endpoint (@@1234567890)
	pub fn new(name_binary:&Vec<u8>, subspaces:Vec<String>) -> Endpoint {
		Endpoint {
			name: buffers::buffer_to_hex(name_binary.to_vec()),
			name_binary: Some(name_binary.to_vec()),
			endpoint_type: EndpointType::Id,
			subspaces
		}
	}

	// create alias endpoint (@person)
	pub fn new_person_alias(name:String, subspaces:Vec<String>) -> Endpoint {
		Endpoint {
			name: name,
			name_binary: None,
			endpoint_type: EndpointType::PersonAlias,
			subspaces
		}
	}

	// create institution endpoint (@+institution)
	pub fn new_institution_alias(name:String, subspaces:Vec<String>) -> Endpoint {
		Endpoint {
			name: name,
			name_binary: None,
			endpoint_type: EndpointType::InstitutionAlias,
			subspaces
		}
	}


	pub fn to_string(&self, colorized:bool) -> String {
		let mut main = match self.endpoint_type {
			EndpointType::Id => format!("{}@@{}",				 	(if colorized {Color::ENDPOINT.as_ansi_rgb()} else {"".to_string()}), self.name),
			EndpointType::PersonAlias => format!("{}@{}", 			(if colorized {Color::EndpointPerson.as_ansi_rgb()} else {"".to_string()}), self.name),
			EndpointType::InstitutionAlias => format!("{}@+{}", 	(if colorized {Color::EndpointInstitution.as_ansi_rgb()} else {"".to_string()}), self.name)
		};
		for subspace in &self.subspaces {
			if colorized {main += &Color::DEFAULT.as_ansi_rgb()}
			main += ".";
			if colorized {main += &Color::DefaultLight.as_ansi_rgb()}
			main += subspace;
		}
		return main;
	}

}
use super::Value;

pub struct Type {

	pub namespace: String,
	pub name: String,
	pub variation: Option<String>,

}


impl Value for Type {
    fn to_string(&self) -> String {
		if self.namespace.len() == 0 || self.namespace == "std" {
			if self.variation.is_some() {
				return format!("<{}/{}>", self.name, self.variation.as_ref().unwrap());
			}
			else {
				return format!("<{}>", self.name);
			}
		} 
		else {
			if self.variation.is_some() {
				return format!("<{}:{}/{}>", self.namespace, self.name, self.variation.as_ref().unwrap());
			}
			else {
				return format!("<{}:{}>", self.namespace, self.name);
			}
		}
    }
}


pub mod std_types {
    use lazy_static::lazy_static;
    use super::Type;

	lazy_static!{
		pub static ref SET:Type = Type {namespace:"".to_string(), name:"Set".to_string(), variation:None};
		pub static ref MAP:Type = Type {namespace:"".to_string(), name:"Map".to_string(), variation:None};
	}
}


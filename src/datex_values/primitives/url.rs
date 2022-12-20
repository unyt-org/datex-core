#[derive(Clone)]
pub struct Url {
	pub url: String,
}

impl Url {

    pub fn to_string(&self) -> String {
        return self.url.to_string();
    }

    pub fn new(url:String) -> Url {
        return Url {url}
    }
}
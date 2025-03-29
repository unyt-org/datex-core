#[derive(Clone)]
pub struct Url {
    pub url: String,
}

impl Url {
    pub fn to_string(&self) -> String {
        self.url.to_string()
    }

    pub fn new(url: String) -> Url {
        Url { url }
    }
}

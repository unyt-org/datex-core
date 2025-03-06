use anyhow::{anyhow, Result};
use url::Url;

pub fn parse_url(address: &str) -> Result<Url> {
	let address = if address.contains("://") {
		address.to_string()
	} else {
		format!("wss://{}", address)
	};

	let mut url = Url::parse(&address).map_err(|_| anyhow!("Invalid URL"))?;
	match url.scheme() {
		"https" => url.set_scheme("wss").unwrap(),
		"http" => url.set_scheme("ws").unwrap(),
		"wss" | "ws" => (),
		_ => return Err(anyhow!("Invalid URL scheme")),
	}
	Ok(url)
}
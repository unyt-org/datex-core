#[derive(Clone)]
pub struct Time {
	pub ms: u64,
}

impl Time {

    pub fn to_string(&self) -> String {
        return format!("~ms:{}~", self.ms);
    }

    pub fn from_milliseconds(ms:u64) -> Time {
        return Time {ms}
    }
}
#[derive(Clone)]
pub struct Time {
  pub ms: u64,
}

impl Time {
  pub fn to_string(&self) -> String {
    // TODO: use primitive timestamp representation
    return format!("<time> {}", self.ms);
  }

  pub fn from_milliseconds(ms: u64) -> Time {
    return Time { ms };
  }
}

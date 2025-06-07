use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use super::super::core_value_trait::CoreValueTrait;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Text(pub String);

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl Text {
    pub fn length(&self) -> usize {
        self.0.len()
    }
    pub fn to_uppercase(&self) -> Text {
        Text(self.0.to_uppercase())
    }
    pub fn to_lowercase(&self) -> Text {
        Text(self.0.to_lowercase())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl Text {
    pub fn reverse(&mut self) {
        let reversed = self.0.chars().rev().collect::<String>();
        self.0 = reversed;
    }
}

impl CoreValueTrait for Text {}

/// The froms are used for this magic. This will automatically convert
/// the Rust types to Text when using the += operator.
impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text(s.to_string())
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text(s)
    }
}

/// Allow TypedDatexValue<Text> += String and TypedDatexValue<Text> += &str
/// This can never panic since the Text::from from string will always succeed
impl AddAssign<Text> for Text {
    fn add_assign(&mut self, rhs: Text) {
        self.0 += &rhs.0;
    }
}

impl Add for Text {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Text(self.0 + &rhs.0)
    }
}

impl Add for &Text {
    type Output = Text;

    fn add(self, rhs: Self) -> Self::Output {
        Text(self.0.clone() + &rhs.0)
    }
}

impl Add<Text> for &Text {
    type Output = Text;

    fn add(self, rhs: Text) -> Self::Output {
        Text(self.0.clone() + &rhs.0)
    }
}

impl Add<&Text> for Text {
    type Output = Text;

    fn add(self, rhs: &Text) -> Self::Output {
        Text(self.0 + &rhs.0)
    }
}

impl Add<&str> for Text {
    type Output = Text;

    fn add(self, rhs: &str) -> Self::Output {
        Text(self.0 + rhs)
    }
}

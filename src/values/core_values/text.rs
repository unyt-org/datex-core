use super::super::core_value_trait::CoreValueTrait;
use crate::stdlib::ops::{Add, AddAssign};
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use crate::traits::structural_eq::StructuralEq;
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;
use serde::{Deserialize, Serialize};
use crate::references::reference::IndexOutOfBoundsError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Text(pub String);

impl Display for Text {
    // TODO #319: escape string content
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::write!(f, "\"{}\"", self.0)
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
    pub fn char_at(&self, index: i64) -> Result<char, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        self.0.chars().nth(index).ok_or(IndexOutOfBoundsError { index: index as u32 })
    }
    
    #[inline]
    fn wrap_index(&self, index: i64) -> usize {
        if index < 0 {
            let len = self.0.chars().count() as i64;
            (len + index) as usize
        } else {
            index as usize
        }
    }
    #[inline]
    fn get_valid_index(&self, index: i64) -> Result<usize, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        if (index) < self.0.len() {
            Ok(index)
        } else {
            Err(IndexOutOfBoundsError { index: index as u32 })
        }
    }


    pub fn substring(&self, start: usize, end: usize) -> Option<Text> {
        if start > end || end > self.0.len() {
            return None;
        }
        Some(Text(self.0[start..end].to_string()))
    }
    pub fn contains(&self, substring: &str) -> bool {
        self.0.contains(substring)
    }
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }
    pub fn index_of(&self, substring: &str) -> Option<usize> {
        self.0.find(substring)
    }
    pub fn last_index_of(&self, substring: &str) -> Option<usize> {
        self.0.rfind(substring)
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn trim(&self) -> Text {
        Text(self.0.trim().to_string())
    }
    pub fn trim_start(&self) -> Text {
        Text(self.0.trim_start().to_string())
    }
    pub fn split(&self, delimiter: &str) -> Vec<Text> {
        self.0
            .split(delimiter)
            .map(|s| Text(s.to_string()))
            .collect()
    }
    pub fn join(texts: &[Text], separator: &str) -> Text {
        let joined = texts
            .iter()
            .map(|t| t.0.as_str())
            .collect::<Vec<&str>>()
            .join(separator);
        Text(joined)
    }
    pub fn repeat(&self, n: usize) -> Text {
        Text(self.0.repeat(n))
    }
}

// modifiers
impl Text {
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn reverse(&mut self) {
        let reversed = self.0.chars().rev().collect::<String>();
        self.0 = reversed;
    }
    pub fn push_str(&mut self, s: &str) {
        self.0.push_str(s);
    }
    pub fn push_char(&mut self, c: char) {
        self.0.push(c);
    }
    pub fn pop_char(&mut self) -> Option<char> {
        self.0.pop()
    }
    pub fn insert(&mut self, index: usize, s: &str) -> Result<(), String> {
        if index > self.0.len() {
            return Err("Index out of bounds".to_string());
        }
        self.0.insert_str(index, s);
        Ok(())
    }
    // TODO #320: Add proper error handling, also for insert and other analog to MapAccessError
    pub fn remove(&mut self, index: usize) -> Result<char, String> {
        if index >= self.0.len() {
            return Err("Index out of bounds".to_string());
        }
        Ok(self.0.remove(index))
    }
    pub fn replace(&mut self, from: &str, to: &str) {
        self.0 = self.0.replace(from, to);
    }
    pub fn replace_range(
        &mut self,
        range: core::ops::Range<usize>,
        replace_with: &str,
    ) -> Result<(), String> {
        if range.start > range.end || range.end > self.0.len() {
            return Err("Range out of bounds".to_string());
        }
        self.0.replace_range(range, replace_with);
        Ok(())
    }
    pub fn set_char_at(&mut self, index: i64, c: char) -> Result<(), IndexOutOfBoundsError> {
        let index = self.get_valid_index(index)?;
        let mut chars: Vec<char> = self.0.chars().collect();
        chars[index] = c;
        self.0 = chars.iter().collect();
        Ok(())
    }
}

impl CoreValueTrait for Text {}

impl StructuralEq for Text {
    fn structural_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

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

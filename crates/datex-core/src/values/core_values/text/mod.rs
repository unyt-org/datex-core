//! This module contains the implementation of the [Text] struct, which represents a string value in the type system.
use crate::{
    prelude::*,
    shared_values::errors::IndexOutOfBoundsError,
};
use core::{fmt::Display, result::Result};
pub mod equality;
mod to_instructions;

#[derive(Debug, Clone)]
pub enum Text {
    /// Represents a dynamically allocated string.
    String(String),
    /// Represents a statically allocated string slice.
    StaticStr(&'static str),
}
mod classification;
mod convert_parts;
pub mod datex_hash;
mod datex_native;
mod datex_native_structural;
mod get_core_lib_type_id;
mod get_datex_type;
pub mod ops;
pub mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod try_clone;
mod value_access;
pub mod binrw;
pub mod hash;

impl Display for Text {
    // TODO #319: escape string content
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::write!(f, "\"{}\"", self.as_str())
    }
}

impl Text {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Text::String(s.into())
    }
    pub fn len(&self) -> usize {
        match self {
            Text::String(s) => s.len(),
            Text::StaticStr(s) => s.len(),
        }
    }
    pub fn to_uppercase(&self) -> Text {
        match self {
            Text::String(s) => Text::String(s.to_uppercase()),
            Text::StaticStr(s) => Text::String(s.to_uppercase()),
        }
    }
    pub fn to_lowercase(&self) -> Text {
        match self {
            Text::String(s) => Text::String(s.to_lowercase()),
            Text::StaticStr(s) => Text::String(s.to_lowercase()),
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Text::String(s) => s.as_str(),
            Text::StaticStr(s) => s,
        }
    }

    pub fn try_as_mut_str(&mut self) -> Result<&mut str, ()> {
        match self {
            Text::String(s) => Ok(s.as_mut_str()),
            Text::StaticStr(_) => Err(()),
        }
    }

    /// Converts the Text into a String, cloning the inner string if necessary.
    pub fn as_string(&self) -> String {
        match self {
            Text::String(s) => s.clone(),
            Text::StaticStr(s) => s.to_string(),
        }
    }

    /// Converts the Text into a String, consuming the Text in the process.
    /// If the Text is a StaticStr variant, it will allocate a new String from the static string slice.
    pub fn into_string(self) -> String {
        match self {
            Text::String(s) => s,
            Text::StaticStr(s) => s.to_string(),
        }
    }

    pub fn char_at(&self, index: u32) -> Result<char, IndexOutOfBoundsError> {
        match self {
            Text::String(s) => s.chars().nth(index as usize).ok_or(IndexOutOfBoundsError { index }),
            Text::StaticStr(s) => s.chars().nth(index as usize).ok_or(IndexOutOfBoundsError { index }),
        }
    }

    #[inline]
    fn wrap_index(&self, index: i64) -> usize {
        if index < 0 {
            let len = match self {
                Text::String(s) => s.chars().count() as i64,
                Text::StaticStr(s) => s.chars().count() as i64,
            };
            (len + index) as usize
        } else {
            index as usize
        }
    }
    #[inline]
    fn get_valid_index(
        &self,
        index: i64,
    ) -> Result<usize, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        if (index) < self.as_str().len() {
            Ok(index)
        } else {
            Err(IndexOutOfBoundsError {
                index: index as u32,
            })
        }
    }

    pub fn substring(&self, start: usize, end: usize) -> Option<Text> {
        if start > end || end > self.len() {
            return None;
        }
        Some(Text::String(self.as_str()[start..end].to_string()))
    }
    pub fn contains(&self, substring: &str) -> bool {
        self.as_str().contains(substring)
    }
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.as_str().starts_with(prefix)
    }
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.as_str().ends_with(suffix)
    }
    pub fn index_of(&self, substring: &str) -> Option<usize> {
        self.as_str().find(substring)
    }
    pub fn last_index_of(&self, substring: &str) -> Option<usize> {
        self.as_str().rfind(substring)
    }
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
    pub fn trim(&self) -> Text {
        Text::String(self.as_str().trim().to_string())
    }
    pub fn trim_start(&self) -> Text {
        Text::String(self.as_str().trim_start().to_string())
    }
    pub fn split(&self, delimiter: &str) -> Vec<Text> {
        self.as_str()
            .split(delimiter)
            .map(|s| Text::String(s.to_string()))
            .collect()
    }
    pub fn join(texts: &[Text], separator: &str) -> Text {
        let joined = texts
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<&str>>()
            .join(separator);
        Text::String(joined)
    }
    pub fn repeat(&self, n: usize) -> Text {
        Text::String(self.as_str().repeat(n))
    }
}

// modifiers
impl Text {
    /// Returns a mutable reference to the inner String if the Text is a String variant, otherwise returns an error.
    /// This allows for modifying the inner String directly, but will fail if the Text is a StaticStr variant.
    pub fn try_as_mut_string(&mut self) -> Result<&mut String, ()> {
        match self {
            Text::String(s) => Ok(s),
            Text::StaticStr(_) => Err(()),
        }
    }
    pub fn clear(&mut self) -> Result<(), ()> {
        self.try_as_mut_string().map(|s| s.clear())
    }
    pub fn reverse(&mut self) -> Result<(), ()> {
        self.try_as_mut_string().map(|s| {
            let reversed = s.chars().rev().collect::<String>();
            *s = reversed;
        })
    }
    pub fn push_str(&mut self, s: &str) -> Result<(), ()> {
        self.try_as_mut_string().map(|inner| inner.push_str(s))
    }
    pub fn push_char(&mut self, c: char) -> Result<(), ()> {
        self.try_as_mut_string().map(|inner| inner.push(c))
    }
    pub fn pop_char(&mut self) -> Result<Option<char>, ()> {
        self.try_as_mut_string().map(|inner| inner.pop())
    }

    /// Inserts a string slice at the specified index in the Text.
    /// Returns an error if the Text is a StaticStr variant or if the index is out of bounds.
    pub fn insert(&mut self, index: usize, s: &str) -> Result<(), ()> {
        self.try_as_mut_string().map(|inner| {
            if index > inner.len() {
                return Err(())
            }
            inner.insert_str(index, s);
            Ok(())
        }).flatten()
    }

    // TODO #320: Add proper error handling, also for insert and other analog to MapAccessError
    pub fn remove(&mut self, index: usize) -> Result<char, ()> {
        self.try_as_mut_string().map(|inner| {
            if index >= inner.len() {
                return Err(())
            }
            Ok(inner.remove(index))
        }).flatten()
    }
    pub fn replace(&mut self, from: &str, to: &str) -> Result<(), ()> {
        self.try_as_mut_string().map(|inner| {
            *inner = inner.replace(from, to);
            Ok(())
        }).flatten()
    }
    pub fn replace_range(
        &mut self,
        range: core::ops::Range<usize>,
        replace_with: &str,
    ) -> Result<(), ()> {
        self.try_as_mut_string().map(|inner| {
            if range.start > range.end || range.end > inner.len() {
                return Err(())
            }
            inner.replace_range(range, replace_with);
            Ok(())
        }).flatten()
    }
    pub fn set_char_at(
        &mut self,
        index: u32,
        c: char,
    ) -> Result<(), SetCharAtError> {
        self.try_as_mut_string()
            .map(|inner| {
                let mut chars: Vec<char> = inner.chars().collect();
                chars[index as usize] = c;
                *inner = chars.iter().collect();
                Ok(())
            })
            .map_err(|_| SetCharAtError::NotMutableError)
            .flatten()
    }
}

pub enum SetCharAtError {
    IndexOutOfBoundsError(IndexOutOfBoundsError),
    NotMutableError,
}


/// The froms are used for this magic. This will automatically convert
/// the Rust types to Text when using the += operator.
impl From<&'static str> for Text {
    fn from(s: &'static str) -> Self {
        Text::StaticStr(s)
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::String(s)
    }
}

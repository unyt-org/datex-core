use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct TypePath<'a> {
    pub namespace: &'a str,
    pub name: &'a str,
    pub variant: Option<&'a str>,
}

impl<'a> TypePath<'a> {
    pub fn new<N: Into<&'a str>, T: Into<&'a str>>(
        namespace: N,
        name: T,
        variant: Option<&'a str>,
    ) -> Self {
        TypePath {
            namespace: namespace.into(),
            name: name.into(),
            variant,
        }
    }

    pub fn parse(s: &'a str) -> Self {
        // split namespace:type
        let mut parts = s.splitn(2, ':');
        let namespace =
            parts.next().expect("namespace is required").to_string();
        let rest = parts.next().expect("type is required");

        // split type/variant
        let mut tv_parts = rest.splitn(2, '/');
        let name = tv_parts.next().unwrap().to_string();
        let variant = tv_parts.next().map(|s| s.to_string());

        TypePath {
            namespace,
            name,
            variant: variant.as_deref(),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        if self.variant.is_some() {
            Some(TypePath::new(&self.namespace, &self.name, None))
        } else {
            None
        }
    }

    pub fn is_parent_of(&self, other: &Self) -> bool {
        self.namespace == other.namespace
            && self.name == other.name
            && self.variant.is_none()
            && other.variant.is_some()
    }

    pub fn as_str(&self) -> String {
        match &self.variant {
            Some(v) => format!("{}:{}/{}", self.namespace, self.name, v),
            None => format!("{}:{}", self.namespace, self.name),
        }
    }
}

impl<'a> From<&'a str> for TypePath<'a> {
    fn from(s: &'a str) -> Self {
        TypePath::parse(s)
    }
}

impl<'a, N: Into<&'a str>, T: Into<&'a str>> From<(N, T)> for TypePath<'a> {
    fn from(value: (N, T)) -> Self {
        TypePath::new(value.0, value.1, None)
    }
}

impl<'a, N: Into<&'a str>, T: Into<&'a str>> From<(N, T, &'a str)>
    for TypePath<'a>
{
    fn from(value: (N, T, &'a str)) -> Self {
        TypePath::new(value.0, value.1, Some(value.2))
    }
}

impl<'a> Display for TypePath<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_type_path() {
        let path = TypePath::new("std", "integer", None);
        assert_eq!(path.namespace, "std");
        assert_eq!(path.name, "integer");
        assert!(path.variant.is_none());

        let path_with_variant = TypePath::new("std", "integer", Some("u8"));
        assert_eq!(path_with_variant.namespace, "std");
        assert_eq!(path_with_variant.name, "integer");
        assert_eq!(path_with_variant.variant, Some("u8"));
    }

    #[test]
    fn test_type_path_parsing() {
        let parsed_path = TypePath::parse("std:integer/u8");
        assert_eq!(parsed_path.namespace, "std");
        assert_eq!(parsed_path.name, "integer");
        assert_eq!(parsed_path.variant, Some("u8"));

        let parsed_path = TypePath::parse("std:integer");
        assert_eq!(parsed_path.namespace, "std");
        assert_eq!(parsed_path.name, "integer");
        assert_eq!(parsed_path.variant, None);
    }

    #[test]
    fn test_type_path_parent() {
        let path = TypePath::new("std", "integer", Some("u8"));
        let parent = path.parent().unwrap();
        assert_eq!(parent.namespace, "std");
        assert_eq!(parent.name, "integer");
        assert!(parent.variant.is_none());
    }

    #[test]
    fn test_type_path_is_parent_of() {
        let parent = TypePath::new("std", "integer", None);
        let child = TypePath::new("std", "integer", Some("u8"));
        assert!(parent.is_parent_of(&child));
    }

    #[test]
    fn test_type_path_display() {
        let path = TypePath::new("std", "integer", Some("u8"));
        assert_eq!(path.to_string(), "std:integer/u8");
    }
}

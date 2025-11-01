use internment::Intern;
use core::fmt;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SrcId(Intern<Vec<String>>);

impl fmt::Display for SrcId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_empty() {
            core::write!(f, "?")
        } else {
            core::write!(f, "{}", self.0.clone().join("/"))
        }
    }
}

impl fmt::Debug for SrcId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{}", self)
    }
}

impl SrcId {
    #[cfg(test)]
    pub fn empty() -> Self {
        SrcId(Intern::new(Vec::new()))
    }

    pub fn repl() -> Self {
        SrcId(Intern::new(vec!["repl".to_string()]))
    }
    pub fn test() -> Self {
        SrcId(Intern::new(vec!["test".to_string()]))
    }

    #[cfg(feature = "std")]
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Self {
        SrcId(Intern::new(
            path.as_ref()
                .iter()
                .map(|c| c.to_string_lossy().into_owned())
                .collect(),
        ))
    }

    #[cfg(feature = "std")]
    pub fn to_path(&self) -> std::path::PathBuf {
        self.0.iter().map(|e| e.to_string()).collect()
    }
}
impl Default for SrcId {
    fn default() -> Self {
        SrcId(Intern::new(vec!["<unknown>".to_string()]))
    }
}
impl From<&str> for SrcId {
    fn from(s: &str) -> Self {
        SrcId(Intern::new(vec![s.to_string()]))
    }
}
impl From<String> for SrcId {
    fn from(s: String) -> Self {
        SrcId(Intern::new(vec![s]))
    }
}

#[cfg(feature = "std")]
impl From<std::path::PathBuf> for SrcId {
    fn from(s: std::path::PathBuf) -> Self {
        SrcId::from_path(s)
    }
}

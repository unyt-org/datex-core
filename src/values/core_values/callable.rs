use serde::Serialize;
use crate::serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Callable {
    Function(),
    Procedure()
}
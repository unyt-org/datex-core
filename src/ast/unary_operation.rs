#[derive(Clone, Debug, PartialEq, Copy)]
pub enum UnaryOperator {
    Not,
    Minus,
    Plus,
    CreateRef,
    CreateRefMut,
}

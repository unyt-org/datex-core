use crate::ast::error::error::{ParseError, SpanOrToken};
use crate::ast::tree::DatexExpression;
use std::fmt::{Display, Formatter};
use std::ops::Range;
use chumsky::prelude::SimpleSpan;
use datex_core::compiler::type_inference::SpannedTypeError;
use crate::compiler::precompiler::RichAst;
use crate::compiler::type_inference::{DetailedTypeErrors, TypeError};
use crate::serde::error::DeserializationError;

#[derive(Debug, Clone)]
pub enum CompilerError {
    UnexpectedTerm(Box<DatexExpression>),
    ParseErrors(Vec<ParseError>),
    SerializationError,
    // TODO: SerializationError(binrw::Error),? has no clone
    BigDecimalOutOfBoundsError,
    IntegerOutOfBoundsError,
    InvalidPlaceholderCount,
    NonStaticValue,
    UndeclaredVariable(String),
    InvalidRedeclaration(String),
    SubvariantNotFound(String, String),
    ScopePopError,
    InvalidSlotName(String),
    AssignmentToConst(String),
    AssignmentToImmutableReference(String),
    AssignmentToImmutableValue(String),
    OnceScopeUsedMultipleTimes,
    TypeError(TypeError),
    ParseError(ParseError),
}

/// A compiler error that can be linked to a specific span in the source code
#[derive(Debug)]
pub struct SpannedCompilerError {
    pub error: CompilerError,
    pub span: Option<Range<usize>>
}

impl SpannedCompilerError {
    pub fn new_with_simple_span(error: CompilerError, span: SimpleSpan) -> SpannedCompilerError {
        SpannedCompilerError {
            error,
            span: Some(span.start..span.end)
        }
    }
}

impl Display for SpannedCompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.span.as_ref().map(|s|format!("{}..{}", s.start, s.end)).unwrap_or("?".to_string()))
    }
}

impl From<SpannedTypeError> for SpannedCompilerError {
    fn from(value: SpannedTypeError) -> Self {
        SpannedCompilerError {
            span: value.span,
            error: value.error.into(),
        }
    }
}

impl From<ParseError> for SpannedCompilerError {
    fn from(value: ParseError) -> Self {
        SpannedCompilerError {
            span: match &value.span {
                SpanOrToken::Span(range) => Some(range.clone()),
                _ => panic!("expected byte range, got token span")
            },
            error: CompilerError::ParseError(value),
        }
    }
}

impl From<CompilerError> for SpannedCompilerError {
    fn from(value: CompilerError) -> Self {
        SpannedCompilerError {
            error: value,
            span: None
        }
    }
}

impl From<SpannedCompilerError> for DeserializationError {
    fn from(e: SpannedCompilerError) -> Self {
        DeserializationError::CompilerError(e)
    }
}


#[derive(Debug, Default)]
pub struct DetailedCompilerErrors {
    pub errors: Vec<SpannedCompilerError>
}

impl DetailedCompilerErrors {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn append(&mut self, mut errors: DetailedCompilerErrors) {
        self.errors.append(&mut errors.errors);
    }
}

impl Display for DetailedCompilerErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for error in self.errors.iter() {
            write!(f, "{}\n", error)?;
        }
        Ok(())
    }
}

impl DetailedCompilerErrors {
    pub fn record_error_with_span(&mut self, error: CompilerError, span: Range<usize>) {
        self.record_error(SpannedCompilerError { error, span: Some(span) });
    }
}

impl From<DetailedTypeErrors> for DetailedCompilerErrors {
    fn from(value: DetailedTypeErrors) -> Self {
        DetailedCompilerErrors {
            errors: value
                .errors
                .into_iter()
                .map(SpannedCompilerError::from)
                .collect()
        }
    }
}

impl ErrorCollector<SpannedCompilerError> for DetailedCompilerErrors {
    fn record_error(&mut self, error: SpannedCompilerError) {
        self.errors.push(error);
    }

}

#[derive(Debug)]
pub enum SimpleOrDetailedCompilerError {
    Simple(SpannedCompilerError),
    Detailed(DetailedCompilerErrors),
}

impl From<CompilerError> for SimpleOrDetailedCompilerError {
    fn from(value: CompilerError) -> Self {
        SimpleOrDetailedCompilerError::Simple(SpannedCompilerError::from(value))
    }
}

impl From<SpannedCompilerError> for SimpleOrDetailedCompilerError {
    fn from(value: SpannedCompilerError) -> Self {
        SimpleOrDetailedCompilerError::Simple(value)
    }
}



#[derive(Debug)]
pub struct DetailedCompilerErrorsWithRichAst {
    pub errors: DetailedCompilerErrors,
    pub ast: RichAst
}

#[derive(Debug)]
pub struct DetailedCompilerErrorsWithMaybeRichAst {
    pub errors: DetailedCompilerErrors,
    pub ast: Option<RichAst>
}

impl From<DetailedCompilerErrorsWithRichAst> for DetailedCompilerErrorsWithMaybeRichAst {
    fn from(value: DetailedCompilerErrorsWithRichAst) -> Self {
        DetailedCompilerErrorsWithMaybeRichAst {
            errors: value.errors,
            ast: Some(value.ast)
        }
    }
}

/// Extended SimpleOrDetailedCompilerError type
/// that includes RichAst for the Detailed variant
#[derive(Debug)]
pub enum SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst {
    /// DetailedCompilerError with additional RichAst
    Detailed(DetailedCompilerErrorsWithRichAst),
    /// simple SpannedCompilerError
    Simple(SpannedCompilerError)
}

impl From<SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst> for SimpleOrDetailedCompilerError {
    fn from(value: SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst) -> Self {
        match value {
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(error) => SimpleOrDetailedCompilerError::Simple(error),
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Detailed(error_with_ast) => SimpleOrDetailedCompilerError::Detailed(error_with_ast.errors)
        }
    }
}


impl From<Vec<ParseError>> for DetailedCompilerErrors {
    fn from(value: Vec<ParseError>) -> Self {
        DetailedCompilerErrors {
            errors: value
                .into_iter()
                .map(SpannedCompilerError::from)
                .collect()
        }
    }
}

impl From<TypeError> for SimpleOrDetailedCompilerError {
    fn from(value: TypeError) -> Self {
        // TODO: also store and map span from type error
        SimpleOrDetailedCompilerError::Simple(SpannedCompilerError::from(CompilerError::from(value)))
    }
}

impl From<TypeError> for CompilerError {
    fn from(value: TypeError) -> Self {
        CompilerError::TypeError(value)
    }
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::InvalidRedeclaration(name) => {
                write!(f, "Invalid redeclaration of {name}")
            }
            CompilerError::UnexpectedTerm(rule) => {
                write!(f, "Unexpected term: {rule:?}")
            }
            CompilerError::ParseErrors(error) => {
                for e in error {
                    writeln!(f, "{}", e.message())?;
                }
                Ok(())
            }
            CompilerError::SubvariantNotFound(name, variant) => {
                write!(f, "Subvariant {variant} does not exist for {name}")
            }
            CompilerError::SerializationError => {
                write!(f, "Serialization error")
            }
            CompilerError::BigDecimalOutOfBoundsError => {
                write!(f, "BigDecimal out of bounds error")
            }
            CompilerError::IntegerOutOfBoundsError => {
                write!(f, "Integer out of bounds error")
            }
            CompilerError::InvalidPlaceholderCount => {
                write!(f, "Invalid placeholder count")
            }
            CompilerError::NonStaticValue => {
                write!(f, "Encountered non-static value")
            }
            CompilerError::UndeclaredVariable(var) => {
                write!(f, "Undeclared variable: {var}")
            }
            CompilerError::ScopePopError => {
                write!(f, "Could not pop scope, stack is empty")
            }
            CompilerError::InvalidSlotName(name) => {
                write!(f, "Slot #{name} does not exist")
            }
            CompilerError::AssignmentToConst(name) => {
                write!(f, "Cannot assign new value to const {name}")
            }
            CompilerError::OnceScopeUsedMultipleTimes => {
                write!(
                    f,
                    "Scope cannot be used multiple times, set 'once' to false to use a scope multiple times"
                )
            }
            CompilerError::AssignmentToImmutableValue(name) => {
                write!(f, "Cannot assign to immutable value: {name}")
            }
            CompilerError::AssignmentToImmutableReference(name) => {
                write!(f, "Cannot assign to immutable reference: {name}")
            }
            CompilerError::TypeError(err) => {
                write!(f, "{}", err)
            }
            CompilerError::ParseError(err) => {
                write!(f, "{:?}", err)
            }
        }
    }
}



/// Describes an optional action that is only executed if an Ok result
/// was returned (used in collect_or_pass_error);
pub enum MaybeAction<T> {
    // optional action should not be performed
    Skip,
    // action should be performed with the provided value
    Do(T)
}

pub trait ErrorCollector<E> {
    fn record_error(&mut self, error: E);
}

/// Handles a generic Result with an SpannedCompilerError error.
/// If the result is Ok(), an Ok(MaybeAction::Do) with the result is returned
/// If result is Error() and collected_errors is Some, the error is appended to the collected_errors
/// and an Ok(MaybeAction::Skip) is returned
/// If result is Error() and collected_errors is None, the error is directly returned
pub fn collect_or_pass_error<T, E, Collector: ErrorCollector<E>>(
    collected_errors: &mut Option<Collector>,
    result: Result<T, E>,
) -> Result<MaybeAction<T>, E> {
    if let Ok(result) = result {
        Ok(MaybeAction::Do(result))
    }
    else {
        let error = unsafe { result.unwrap_err_unchecked() };
        if let Some(collected_errors) = collected_errors {
            collected_errors.record_error(error);
            Ok(MaybeAction::Skip)
        }
        else {
            Err(error)
        }
    }
}


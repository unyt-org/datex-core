use crate::ast::expressions::DatexExpression;
use crate::compiler::precompiler::precompiled_ast::RichAst;
use crate::parser::errors::{ParserError, SpannedParserError};
use crate::serde::error::DeserializationError;
use crate::type_inference::error::{
    DetailedTypeErrors, SpannedTypeError, TypeError,
};
use core::fmt::{Display, Formatter};
use core::ops::Range;

#[derive(Debug, Clone)]
pub enum CompilerError {
    UnexpectedTerm(Box<DatexExpression>),
    SerializationError,
    // TODO #478: SerializationError(binrw::Error),? has no clone
    BigDecimalOutOfBoundsError,
    IntegerOutOfBoundsError,
    InvalidPlaceholderCount,
    TooManyApplyArguments, // more than 255 arguments
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
    ParserError(ParserError),
}

/// A compiler error that can be linked to a specific span in the source code
#[derive(Debug)]
pub struct SpannedCompilerError {
    pub error: CompilerError,
    pub span: Option<Range<usize>>,
}

impl SpannedCompilerError {
    pub fn new_with_span(
        error: CompilerError,
        span: Range<usize>,
    ) -> SpannedCompilerError {
        SpannedCompilerError {
            error,
            span: Some(span),
        }
    }
}

impl Display for SpannedCompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        core::write!(
            f,
            "{} ({})",
            self.error,
            self.span
                .as_ref()
                .map(|s| format!("{}..{}", s.start, s.end))
                .unwrap_or("?".to_string())
        )
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

impl From<SpannedParserError> for SpannedCompilerError {
    fn from(value: SpannedParserError) -> Self {
        SpannedCompilerError {
            span: Some(value.span),
            error: CompilerError::ParserError(value.error),
        }
    }
}

impl From<CompilerError> for SpannedCompilerError {
    fn from(value: CompilerError) -> Self {
        SpannedCompilerError {
            error: value,
            span: None,
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
    pub errors: Vec<SpannedCompilerError>,
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
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for error in self.errors.iter() {
            writeln!(f, "{}", error)?;
        }
        Ok(())
    }
}

impl DetailedCompilerErrors {
    pub fn record_error_with_span(
        &mut self,
        error: CompilerError,
        span: Range<usize>,
    ) {
        self.record_error(SpannedCompilerError {
            error,
            span: Some(span),
        });
    }
}

impl From<DetailedTypeErrors> for DetailedCompilerErrors {
    fn from(value: DetailedTypeErrors) -> Self {
        DetailedCompilerErrors {
            errors: value
                .errors
                .into_iter()
                .map(SpannedCompilerError::from)
                .collect(),
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
    pub ast: RichAst,
}

#[derive(Debug)]
pub struct DetailedCompilerErrorsWithMaybeRichAst {
    pub errors: DetailedCompilerErrors,
    pub ast: Option<RichAst>,
}

impl From<DetailedCompilerErrorsWithRichAst>
    for DetailedCompilerErrorsWithMaybeRichAst
{
    fn from(value: DetailedCompilerErrorsWithRichAst) -> Self {
        DetailedCompilerErrorsWithMaybeRichAst {
            errors: value.errors,
            ast: Some(value.ast),
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
    Simple(SpannedCompilerError),
}

impl From<SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst>
    for SimpleOrDetailedCompilerError
{
    fn from(
        value: SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst,
    ) -> Self {
        match value {
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(
                error,
            ) => SimpleOrDetailedCompilerError::Simple(error),
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Detailed(
                error_with_ast,
            ) => SimpleOrDetailedCompilerError::Detailed(error_with_ast.errors),
        }
    }
}

impl From<SpannedCompilerError>
    for SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst
{
    fn from(
        value: SpannedCompilerError,
    ) -> SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst {
        SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(value)
    }
}

impl From<Vec<SpannedParserError>> for DetailedCompilerErrors {
    fn from(value: Vec<SpannedParserError>) -> Self {
        DetailedCompilerErrors {
            errors: value.into_iter().map(SpannedCompilerError::from).collect(),
        }
    }
}

impl From<TypeError> for SimpleOrDetailedCompilerError {
    fn from(value: TypeError) -> Self {
        // TODO #479: also store and map span from type error
        SimpleOrDetailedCompilerError::Simple(SpannedCompilerError::from(
            CompilerError::from(value),
        ))
    }
}

impl From<TypeError> for CompilerError {
    fn from(value: TypeError) -> Self {
        CompilerError::TypeError(value)
    }
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CompilerError::InvalidRedeclaration(name) => {
                core::write!(f, "Invalid redeclaration of {name}")
            }
            CompilerError::UnexpectedTerm(rule) => {
                core::write!(f, "Unexpected term: {rule:?}")
            }
            CompilerError::SubvariantNotFound(name, variant) => {
                core::write!(
                    f,
                    "Subvariant {variant} does not exist for {name}"
                )
            }
            CompilerError::SerializationError => {
                core::write!(f, "Serialization error")
            }
            CompilerError::BigDecimalOutOfBoundsError => {
                core::write!(f, "BigDecimal out of bounds error")
            }
            CompilerError::IntegerOutOfBoundsError => {
                core::write!(f, "Integer out of bounds error")
            }
            CompilerError::InvalidPlaceholderCount => {
                core::write!(f, "Invalid placeholder count")
            }
            CompilerError::NonStaticValue => {
                core::write!(f, "Encountered non-static value")
            }
            CompilerError::UndeclaredVariable(var) => {
                core::write!(f, "Undeclared variable: {var}")
            }
            CompilerError::ScopePopError => {
                core::write!(f, "Could not pop scope, stack is empty")
            }
            CompilerError::InvalidSlotName(name) => {
                core::write!(f, "Slot #{name} does not exist")
            }
            CompilerError::AssignmentToConst(name) => {
                core::write!(f, "Cannot assign new value to const {name}")
            }
            CompilerError::OnceScopeUsedMultipleTimes => {
                core::write!(
                    f,
                    "Scope cannot be used multiple times, set 'once' to false to use a scope multiple times"
                )
            }
            CompilerError::AssignmentToImmutableValue(name) => {
                core::write!(f, "Cannot assign to immutable value: {name}")
            }
            CompilerError::AssignmentToImmutableReference(name) => {
                core::write!(f, "Cannot assign to immutable reference: {name}")
            }
            CompilerError::TypeError(err) => {
                core::write!(f, "{}", err)
            }
            CompilerError::ParserError(err) => {
                core::write!(f, "{:?}", err)
            }
            CompilerError::TooManyApplyArguments => {
                core::write!(
                    f,
                    "Apply has too many arguments (max 255 allowed)"
                )
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
    Do(T),
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
    if let Err(error) = result {
        if let Some(collected_errors) = collected_errors {
            collected_errors.record_error(error);
            Ok(MaybeAction::Skip)
        } else {
            Err(error)
        }
    } else {
        result.map(MaybeAction::Do)
    }
}

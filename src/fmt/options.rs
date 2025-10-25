#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormattingOptions {
    /// Number of spaces to use for indentation.
    pub indent: usize,

    /// Maximum line width before wrapping occurs.
    pub max_width: usize,

    /// Whether to add trailing commas in collections like lists and maps.
    /// E.g., `[1, 2, 3,]` instead of `[1, 2, 3]`.
    pub trailing_comma: bool,

    /// Whether to add spaces inside brackets of collections like lists and maps.
    /// E.g., `[ 1,2,3 ]` instead of `[1,2,3]`.
    pub spaced_collections: bool,

    /// Whether to add spaces inside collections like lists and maps.
    /// E.g., `[1, 2, 3]` instead of `[1,2,3]`.
    pub space_in_collection: bool,

    /// Whether to add spaces around operators.
    /// E.g., `1 + 2` instead of `1+2`.
    pub spaces_around_operators: bool,

    /// Formatting style for type declarations.
    /// Determines how type annotations are spaced and aligned.
    pub type_declaration_formatting: TypeDeclarationFormatting,

    /// Whether to add newlines between statements.
    pub statement_formatting: StatementFormatting,

    /// Formatting style for type variant suffixes.
    pub variant_formatting: VariantFormatting,

    /// Bracketing style for expressions.
    pub bracket_style: BracketStyle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BracketStyle {
    /// Keep original bracketing as is.
    KeepAll,

    /// Remove only redundant or duplicate outer brackets, e.g. `((42))` -> `(42)`.
    RemoveDuplicate,

    /// Remove all unnecessary brackets based purely on operator precedence.
    Minimal,
}

/// Formatting styles for enum variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantFormatting {
    /// Keep the original formatting.
    KeepAll,
    /// Use variant suffixes.
    WithSuffix,
    /// Do not use variant suffixes.
    WithoutSuffix,
}

/// Formatting styles for statements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementFormatting {
    /// Add a newline between statements.
    NewlineBetween,
    /// Add a space between statements.
    SpaceBetween,
    /// Compact formatting without extra spaces or newlines.
    Compact,
}

/// Formatting styles for type declarations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeDeclarationFormatting {
    /// Compact formatting without extra spaces.
    Compact,
    /// Spaces around the colon in type declarations.
    SpaceAroundColon,
    /// Space after the colon in type declarations.
    SpaceAfterColon,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        FormattingOptions {
            indent: 4,
            max_width: 40,
            variant_formatting: VariantFormatting::KeepAll,
            trailing_comma: true,
            spaced_collections: false,
            space_in_collection: true,
            spaces_around_operators: true,
            type_declaration_formatting:
                TypeDeclarationFormatting::SpaceAfterColon,
            statement_formatting: StatementFormatting::NewlineBetween,
            bracket_style: BracketStyle::Minimal,
        }
    }
}
impl FormattingOptions {
    pub fn compact() -> Self {
        FormattingOptions {
            indent: 2,
            max_width: 40,
            variant_formatting: VariantFormatting::WithoutSuffix,
            trailing_comma: false,
            spaced_collections: false,
            space_in_collection: false,
            spaces_around_operators: false,
            type_declaration_formatting: TypeDeclarationFormatting::Compact,
            statement_formatting: StatementFormatting::Compact,
            bracket_style: BracketStyle::Minimal,
        }
    }
}

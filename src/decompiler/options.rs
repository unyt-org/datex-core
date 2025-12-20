#[derive(Debug, Clone, Default)]
pub struct DecompileOptions {
    pub formatting_options: FormattingOptions,
    /// display slots with generated variable names
    pub resolve_slots: bool,
}

impl DecompileOptions {
    pub fn json_compat() -> Self {
        DecompileOptions {
            formatting_options: FormattingOptions::json_compat(),
            ..DecompileOptions::default()
        }
    }

    /// Formats and colorizes the output
    pub fn colorized() -> Self {
        DecompileOptions {
            formatting_options: FormattingOptions::colorized(),
            ..DecompileOptions::default()
        }
    }

    /// No extra spaces or newlines, no colorization
    pub fn compact() -> Self {
        DecompileOptions {
            formatting_options: FormattingOptions::compact(),
            ..DecompileOptions::default()
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum IndentType {
    Spaces,
    Tabs,
}


#[derive(Debug, Clone)]
pub enum FormattingMode {
    /// compact formatting, no unnecessary spaces or newlines
    Compact,
    /// pretty formatting with indentation and newlines
    Pretty {
        indent: usize,
        indent_type: IndentType,
    },
}

impl Default for FormattingMode {
    /// Default pretty formatting with 4 spaces indentation
    fn default() -> Self {
        FormattingMode::pretty()
    }
}

impl FormattingMode {
    pub fn pretty() -> Self {
        FormattingMode::Pretty { indent: 4, indent_type: IndentType::Spaces }
    }

    pub fn compact() -> Self {
        FormattingMode::Compact
    }

    pub fn pretty_with_indent(indent: usize, indent_type: IndentType) -> Self {
        FormattingMode::Pretty { indent, indent_type }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FormattingOptions {
    pub mode: FormattingMode,
    pub json_compat: bool,
    pub colorized: bool,
    pub add_variant_suffix: bool,
}

impl FormattingOptions {

    pub fn colorized() -> Self {
        FormattingOptions {
            colorized: true,
            ..FormattingOptions::default()
        }
    }

    pub fn json_compat() -> Self {
        FormattingOptions {
            json_compat: true,
            ..FormattingOptions::default()
        }
    }

    pub fn compact() -> Self {
        FormattingOptions {
            mode: FormattingMode::Compact,
            ..FormattingOptions::default()
        }
    }

    pub fn pretty() -> Self {
        FormattingOptions {
            mode: FormattingMode::pretty(),
            ..FormattingOptions::default()
        }
    }

    pub fn pretty_with_indent(indent: usize, indent_type: IndentType) -> Self {
        FormattingOptions {
            mode: FormattingMode::pretty_with_indent(indent, indent_type),
            ..FormattingOptions::default()
        }
    }
}
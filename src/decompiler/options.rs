use crate::serde::Deserialize;
use serde::Serialize;

#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecompileOptions {
    #[serde(default)]
    pub formatting_options: FormattingOptions,
    /// display slots with generated variable names
    #[serde(default)]
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

#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
#[derive(Clone, Debug, Copy, Default, Serialize, Deserialize)]
pub enum IndentType {
    #[default]
    Spaces,
    Tabs,
}

#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FormattingMode {
    /// compact formatting, no unnecessary spaces or newlines
    #[default]
    Compact,
    /// pretty formatting with indentation and newlines
    Pretty {
        indent: usize,
        #[serde(default)]
        indent_type: IndentType,
    },
}

impl FormattingMode {
    pub fn pretty() -> Self {
        FormattingMode::Pretty {
            indent: 4,
            indent_type: IndentType::Spaces,
        }
    }

    pub fn compact() -> Self {
        FormattingMode::Compact
    }

    pub fn pretty_with_indent(indent: usize, indent_type: IndentType) -> Self {
        FormattingMode::Pretty {
            indent,
            indent_type,
        }
    }
}

#[cfg_attr(feature = "wasm_runtime", derive(tsify::Tsify))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormattingOptions {
    #[serde(default)]
    pub mode: FormattingMode,
    #[serde(default)]
    pub json_compat: bool,
    #[serde(default)]
    pub colorized: bool,
    #[serde(default)]
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

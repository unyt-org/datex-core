#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Formatting {
    #[default]
    Compact,
    Multiline {
        indent: usize,
    },
}

#[derive(Debug, Clone, Default)]
pub enum FormattingMode {
    /// compact formatting, no unnecessary spaces or newlines
    Compact,
    /// pretty formatting with indentation and newlines
    #[default]
    Pretty,
}

impl Formatting {
    /// Default multiline formatting with 4 spaces indentation
    pub fn multiline() -> Self {
        Formatting::Multiline { indent: 4 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DecompileOptions {
    pub formatting_mode: FormattingMode,
    pub formatting: Formatting,
    pub colorized: bool,
    /// display slots with generated variable names
    pub resolve_slots: bool,
    /// TODO #224
    /// when set to true, the output is generated as compatible as possible with JSON, e.g. by
    /// always adding double quotes around keys
    pub json_compat: bool,
}

impl DecompileOptions {
    pub fn json() -> Self {
        DecompileOptions {
            json_compat: true,
            ..DecompileOptions::default()
        }
    }

    /// Fomarts and colorizes the output
    pub fn colorized() -> Self {
        DecompileOptions {
            colorized: true,
            formatting: Formatting::Multiline { indent: 4 },
            resolve_slots: true,
            ..DecompileOptions::default()
        }
    }
}

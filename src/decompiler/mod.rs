mod ast_decompiler;
mod ast_from_value_container;
mod ast_to_source_code;

use std::collections::HashMap; // FIXME #222 no-std
use std::collections::HashSet;
use std::fmt::Write;
use std::io::Cursor;
use log::info;
// FIXME #223 no-std

use crate::compiler::{CompileOptions, compile_template_with_refs};
use crate::global::protocol_structures::instructions::Int128Data;
use crate::global::protocol_structures::instructions::IntegerData;
use crate::global::protocol_structures::instructions::UInt8Data;
use crate::global::protocol_structures::instructions::UInt16Data;
use crate::global::protocol_structures::instructions::UInt32Data;
use crate::global::protocol_structures::instructions::UInt64Data;
use crate::global::protocol_structures::instructions::UInt128Data;
use crate::global::protocol_structures::instructions::{
    DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data,
    Instruction, Int8Data, Int16Data, Int32Data, Int64Data, ShortTextData,
    TextData,
};
use crate::parser::body;
use crate::parser::body::DXBParserError;
use crate::values::core_values::decimal::utils::decimal_to_string;
use crate::values::value_container::ValueContainer;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSetBuilder};
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};

/// Decompiles a DXB bytecode body into a human-readable string representation.
pub fn decompile_body(
    dxb_body: &[u8],
    options: DecompileOptions,
) -> Result<String, DXBParserError> {
    let mut initial_state = DecompilerState {
        dxb_body,
        options,

        scopes: vec![ScopeState {
            scope_type: (ScopeType::default(), true),
            ..ScopeState::default()
        }],

        current_label: 0,
        labels: HashMap::new(),
        inserted_labels: HashSet::new(),
        variables: HashMap::new(),
    };

    decompile_loop(&mut initial_state)
}

/// Decompiles a single DATEX value into a human-readable string representation.
pub fn decompile_value(
    value: &ValueContainer,
    options: DecompileOptions,
) -> String {
    let (compiled_value, _) =
        compile_template_with_refs("?", &[value], CompileOptions::default())
            .unwrap();
    decompile_body(&compiled_value, options).unwrap()
}

fn int_to_label(n: i32) -> String {
    // Convert the integer to a base-26 number, with 'a' being the 0th digit
    let mut label = String::new();
    let mut n = n;

    while n > 0 {
        // Get the remainder when n is divided by 26
        let r = n % 26;

        // Add the corresponding character (a-z) to the label
        label.insert(0, (r as u8 + b'a') as char);

        // Divide n by 26 and continue
        n /= 26;
    }

    // If the label is empty, it means the input integer was 0, so return "a"
    if label.is_empty() {
        label = "a".to_string();
    }

    label
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Formatting {
    #[default]
    Compact,
    Indented { indent: usize },
}

impl Formatting {
    pub fn default_indent() -> Self {
        Formatting::Indented { indent: 4 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DecompileOptions {
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
            formatting: Formatting::Indented { indent: 4 },
            resolve_slots: true,
            ..DecompileOptions::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ScopeType {
    #[default]
    Default,
    Tuple,
    Array,
    Object,
    SlotAssignment,
    Transparent,
}

impl ScopeType {
    pub fn write_start(
        &self,
        output: &mut String,
        formatting: &Formatting,
        indentation_levels: usize,
    ) -> Result<(), DXBParserError> {
        match self {
            ScopeType::Default => write!(output, "(")?,
            ScopeType::Tuple => write!(output, "(")?,
            ScopeType::Array => write!(output, "[")?,
            ScopeType::Object => write!(output, "{{")?,
            ScopeType::SlotAssignment => {
                // do nothing, slot assignment does not have a start
            }
            ScopeType::Transparent => {}
        }
        match self {
            ScopeType::Default | ScopeType::Tuple | ScopeType::Array | ScopeType::Object => {
                match formatting {
                    Formatting::Indented { indent } => {
                        write!(output, "\r\n")?;
                        for _ in 0..(indentation_levels * indent) {
                            write!(output, " ")?;
                        }
                    }
                    Formatting::Compact => {}
                }
            },
            _ => {}
        }
        Ok(())
    }
    pub fn write_end(
        &self,
        output: &mut String,
        formatting: &Formatting,
        indentation_levels: usize,
    ) -> Result<(), DXBParserError> {
        match self {
            ScopeType::Default | ScopeType::Tuple | ScopeType::Array | ScopeType::Object => {
                match formatting {
                    Formatting::Indented { indent } => {
                        write!(output, "\r\n")?;
                        for _ in 0..(indentation_levels.saturating_sub(1) * indent) {
                            write!(output, " ")?;
                        }
                    }
                    Formatting::Compact => {}
                }
            },
            _ => {}
        }
        match self {
            ScopeType::Default => write!(output, ")")?,
            ScopeType::Tuple => write!(output, ")")?,
            ScopeType::Array => write!(output, "]")?,
            ScopeType::Object => write!(output, "}}")?,
            ScopeType::SlotAssignment => {
                // do nothing, slot assignment does not have an end
            }
            ScopeType::Transparent => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct ScopeState {
    /// true if this is the outer scope (default scope)
    is_outer_scope: bool,
    // TODO #225: use BinaryOperator instead of Instruction
    active_operator: Option<(Instruction, bool)>,
    scope_type: (ScopeType, bool),
    /// skip inserted comma for next item (already inserted before key)
    skip_comma_for_next_item: bool,
    /// set to true if next item is a key (e.g. in object)
    next_item_is_key: bool,
    /// set to true if the current active scope should be closed after the next term
    close_scope_after_term: bool,
}

impl ScopeState {
    fn write_start(&self, output: &mut String, formatting: &Formatting, indentation_levels: usize) -> Result<(), DXBParserError> {
        self.scope_type.0.write_start(output, formatting, indentation_levels)
    }
    fn write_end(&self, output: &mut String, formatting: &Formatting, indentation_levels: usize) -> Result<(), DXBParserError> {
        self.scope_type.0.write_end(output, formatting, indentation_levels)
    }
}

#[derive(Debug, Clone)]
struct DecompilerState<'a> {
    // stack of scopes
    scopes: Vec<ScopeState>,

    // dxb
    dxb_body: &'a [u8],

    // options
    options: DecompileOptions,

    // state
    current_label: i32,
    labels: HashMap<usize, String>,
    inserted_labels: HashSet<usize>,
    variables: HashMap<u16, String>,
}

impl DecompilerState<'_> {
    fn get_current_scope(&mut self) -> &mut ScopeState {
        self.scopes.last_mut().unwrap()
    }
    fn new_scope(&mut self, scope_type: ScopeType) {
        self.scopes.push(ScopeState {
            scope_type: (scope_type, true),
            ..ScopeState::default()
        });
    }
    fn close_scope(&mut self) {
        if !self.scopes.is_empty() {
            self.scopes.pop();
        }
    }
}

impl DecompilerState<'_> {
    fn get_insert_label(&mut self, index: usize) -> String {
        // existing
        if self.labels.contains_key(&index) {
            self.labels
                .get(&index)
                .unwrap_or(&"?invalid?".to_string())
                .to_string()
        }
        // new
        else {
            let name = self.current_label.to_string();
            self.current_label += 1;
            self.labels.insert(index, name.clone());
            name
        }
    }
}

fn decompile_loop(
    state: &mut DecompilerState,
) -> Result<String, DXBParserError> {
    let mut output = String::new();
    let mut indentation_levels = 0;
    let formatting = state.options.formatting;

    let instruction_iterator = body::iterate_instructions(state.dxb_body);

    for instruction in instruction_iterator {
        let instruction = instruction?;

        match instruction {
            Instruction::Int8(Int8Data(i8)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{i8}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Int16(Int16Data(i16)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{i16}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Int32(Int32Data(i32)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{i32}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Int64(Int64Data(i64)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{i64}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Int128(Int128Data(i128)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{i128}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::UInt8(UInt8Data(u8)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{u8}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::UInt16(UInt16Data(u16)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{u16}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::UInt32(UInt32Data(u32)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{u32}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::UInt64(UInt64Data(u64)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{u64}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::UInt128(UInt128Data(u128)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{u128}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::BigInteger(IntegerData(big_int)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{big_int}n")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalF32(Float32Data(f32)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(f32, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalF64(Float64Data(f64)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(f64, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(i16 as f32, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(i32 as f32, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Decimal(DecimalData(big_decimal)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                write!(output, "{big_decimal}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::ShortText(ShortTextData(text)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                let text = escape_text(&text);
                write!(output, "\"{text}\"")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Text(TextData(text)) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                let text = escape_text(&text);
                write!(output, "\"{text}\"")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::True => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                write!(output, "true")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::False => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                write!(output, "false")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::Null => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                write!(output, "null")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::Endpoint(endpoint) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                write!(output, "{endpoint}")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::TypeTag(tag_data) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                write!(output, "Tag(")?;
                // write tag name
                write!(output, "'{}'", tag_data.name)?;
                // write tag parameters if any
                if !tag_data.variants.is_empty() {
                    write!(output, ",")?;
                    for (i, param) in tag_data.variants.iter().enumerate() {
                        if i > 0 {
                            write!(output, "|")?;
                        }
                        write!(output, "'{}'", param.name)?;
                    }
                }
                write!(output, ")")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::ArrayStart => {
                indentation_levels += 1;
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.new_scope(ScopeType::Array);
                state.get_current_scope().write_start(&mut output, &formatting, indentation_levels)?;
            }
            Instruction::ObjectStart => {
                indentation_levels += 1;
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.new_scope(ScopeType::Object);
                state.get_current_scope().write_start(&mut output, &formatting, indentation_levels)?;
            }
            Instruction::TupleStart => {
                indentation_levels += 1;
                handle_before_term(state, &mut output, true, indentation_levels)?;
                state.new_scope(ScopeType::Tuple);
                state.get_current_scope().write_start(&mut output, &formatting, indentation_levels)?;
            }
            Instruction::ScopeStart => {
                indentation_levels += 1;
                handle_before_term(state, &mut output, true, indentation_levels)?;
                state.new_scope(ScopeType::Default);
                state.get_current_scope().write_start(&mut output, &formatting, indentation_levels)?;
            }
            Instruction::ScopeEnd => {
                let current_scope_is_array_object_tuple = matches!(
                    state.get_current_scope().scope_type.0,
                    ScopeType::Array | ScopeType::Object | ScopeType::Tuple
                );
                handle_scope_close(state, &mut output, indentation_levels)?;
                handle_after_term(state, &mut output, true)?;
                if current_scope_is_array_object_tuple {
                    indentation_levels = indentation_levels.saturating_sub(1);
                }
            }
            Instruction::KeyValueShortText(text_data) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                // prevent redundant comma for value
                state.get_current_scope().skip_comma_for_next_item = true;
                write_text_key(
                    state,
                    &text_data.0,
                    &mut output,
                    state.options.formatting,
                )?;
            }
            Instruction::KeyValueDynamic => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.get_current_scope().skip_comma_for_next_item = true;
                state.get_current_scope().next_item_is_key = true;
            }
            Instruction::CloseAndStore => {
                match state.options.formatting {
                    Formatting::Indented { .. } => {
                        write!(output, ";\r\n")?;
                    }
                    Formatting::Compact => {
                        write!(output, ";")?;
                    }
                }
            }

            // operations
            Instruction::Add
            | Instruction::Subtract
            | Instruction::Multiply
            | Instruction::Divide
            | Instruction::Union => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.new_scope(ScopeType::Transparent);
                state.get_current_scope().active_operator =
                    Some((instruction, true));
            }

            // slots
            Instruction::AllocateSlot(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.new_scope(ScopeType::SlotAssignment);
                // if resolve_slots is enabled, write the slot as variable
                if state.options.resolve_slots {
                    // TODO #95: generate variable name for slot
                    write!(output, "#{} := ", address.0)?;
                } else {
                    // otherwise just write the slot address
                    write!(output, "#{} := ", address.0)?;
                }
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::GetSlot(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                // if resolve_slots is enabled, write the slot as variable
                if state.options.resolve_slots {
                    // TODO #96: get variable name for slot
                    write!(output, "#{}", address.0)?;
                } else {
                    // otherwise just write the slot address
                    write!(output, "#{}", address.0)?;
                }
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::DropSlot(address) => {
                // if resolve_slots is enabled, write the slot as variable
                if state.options.resolve_slots {
                    // TODO #97: generate variable name for slot
                    write!(output, "#drop {}", address.0)?;
                } else {
                    // otherwise just write the slot address
                    write!(output, "#drop {}", address.0)?;
                }
            }
            Instruction::SetSlot(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.new_scope(ScopeType::SlotAssignment);
                // if resolve_slots is enabled, write the slot as variable
                if state.options.resolve_slots {
                    // TODO #98: generate variable name for slot
                    write!(output, "#{} = ", address.0)?;
                } else {
                    // otherwise just write the slot address
                    write!(output, "#{} = ", address.0)?;
                }
            }

            Instruction::GetRef(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                let endpoint_hex = address.endpoint.to_binary().iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                let address_hex = address.id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                write!(output, "$<{}:{}>", endpoint_hex, address_hex)?;
                handle_after_term(state, &mut output, false)?;
            }

            Instruction::GetInternalRef(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                let address_hex = address.id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                write!(output, "$<internal:{}>", address_hex)?;
                handle_after_term(state, &mut output, false)?;
            }

            Instruction::GetOriginRef(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                let address_hex = address.id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                write!(output, "$<origin:{}>", address_hex)?;
                handle_after_term(state, &mut output, false)?;
            }

            Instruction::AddAssign(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.new_scope(ScopeType::SlotAssignment);
                // if resolve_slots is enabled, write the slot as variable
                if state.options.resolve_slots {
                    write!(output, "#{} += ", address.0)?;
                } else {
                    // otherwise just write the slot address
                    write!(output, "#{} += ", address.0)?;
                }
            }

            Instruction::SubtractAssign(address) => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.new_scope(ScopeType::SlotAssignment);
                // if resolve_slots is enabled, write the slot as variable
                if state.options.resolve_slots {
                    write!(output, "#{} -= ", address.0)?;
                } else {
                    // otherwise just write the slot address
                    write!(output, "#{} -= ", address.0)?;
                }
            }

            Instruction::CreateRef => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.get_current_scope().skip_comma_for_next_item = true;
                write!(output, "&")?;
            }

            Instruction::CreateRefMut => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.get_current_scope().skip_comma_for_next_item = true;
                write!(output, "&mut ")?;
            }

            Instruction::RemoteExecution => {
                handle_before_term(state, &mut output, false, indentation_levels)?;
                state.get_current_scope().active_operator =
                    Some((instruction, true));
            }

            Instruction::ExecutionBlock(data) => {
                handle_before_term(state, &mut output, true, indentation_levels)?;
                // decompile data.body
                let decompiled_body =
                    decompile_body(&data.body, state.options.clone())?;
                let slot_mapping = data
                    .injected_slots
                    .iter()
                    .enumerate()
                    .map(|(k, v)| format!("#{v} => #{k}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                // write the decompiled body
                write!(output, "[{slot_mapping}]({decompiled_body})")?;
            }

            _ => {
                write!(output, "[[{instruction}]]")?;
            }
        }
    }

    // add syntax highlighting
    if state.options.colorized {
        output = apply_syntax_highlighting(output)?;
    }

    Ok(output)
}

pub fn apply_syntax_highlighting(
    datex_script: String,
) -> Result<String, DXBParserError> {
    let mut output = String::new();

    // load datex syntax + custom theme
    static DATEX_SCRIPT_DEF: &str = include_str!(
        "../../datex-language/datex.tmbundle/Syntaxes/datex.sublime-text"
    );
    static DATEX_THEME_DEF: &str =
        include_str!("../../datex-language/themes/datex-dark.tmTheme");
    let mut builder = SyntaxSetBuilder::new();
    let syntax = SyntaxDefinition::load_from_str(DATEX_SCRIPT_DEF, true, None)
        .expect("Failed to load syntax definition");
    builder.add(syntax);
    let theme: Theme =
        ThemeSet::load_from_reader(&mut Cursor::new(DATEX_THEME_DEF))
            .expect("Failed to load theme");

    let ps = builder.build();
    let syntax = ps.find_syntax_by_extension("dx").unwrap();
    let mut h = HighlightLines::new(syntax, &theme);

    for line in LinesWithEndings::from(&datex_script) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        write!(output, "{escaped}")?;
    }
    // reset style
    write!(output, "\x1b[0m")?;
    Ok(output)
}

fn escape_text(text: &str) -> String {
    // escape quotes and backslashes in text
    text.replace('\\', r#"\\"#)
        .replace('"', r#"\""#)
        .replace('\u{0008}', r#"\b"#)
        .replace('\u{000c}', r#"\f"#)
        .replace('\r', r#"\r"#)
        .replace('\t', r#"\t"#)
        .replace('\u{000b}', r#"\v"#)
        .replace('\n', r#"\n"#)
}

fn write_text_key(
    state: &mut DecompilerState,
    text: &str,
    output: &mut String,
    formatting: Formatting
) -> Result<(), DXBParserError> {
    // if text does not just contain a-z, A-Z, 0-9, _, and starts with a-z, A-Z,  _, add quotes
    let text = if !state.options.json_compat && is_alphanumeric_identifier(text)
    {
        text.to_string()
    } else {
        format!("\"{}\"", escape_text(text))
    };
    match formatting {
        Formatting::Indented { .. } => {
            write!(output, "{text}: ")?;
        }
        Formatting::Compact => {
            write!(output, "{text}:")?;
        }
    }
    Ok(())
}

fn is_alphanumeric_identifier(s: &str) -> bool {
    let mut chars = s.chars();

    // First character must be a-z, A-Z, or _
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }

    // Remaining characters: a-z, A-Z, 0-9, _, or -
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// insert syntax before a term (e.g. operators, commas, etc.)
/// if is_standalone_key is set to true, no parenthesis are wrapped around the item if it is a key,
/// e.g. for text ("key": "value") the parenthesis are not needed
fn handle_before_term(
    state: &mut DecompilerState,
    output: &mut String,
    is_standalone_key: bool,
    indentation_levels: usize,
) -> Result<(), DXBParserError> {
    handle_before_operand(state, output)?;
    handle_before_item(state, output, is_standalone_key, indentation_levels)?;
    Ok(())
}

/// if is_standalone_key is set to true, no parenthesis are wrapped around the item if it is a key,
/// e.g. for text ("key": "value") the parenthesis are not needed
fn handle_after_term(
    state: &mut DecompilerState,
    output: &mut String,
    is_standalone_key: bool,
) -> Result<(), DXBParserError> {
    let close_scope = state.get_current_scope().close_scope_after_term;
    if close_scope {
        // close scope after term
        state.close_scope();
    }

    // next_item_is_key
    if state.get_current_scope().next_item_is_key {
        if !is_standalone_key || close_scope {
            write!(output, ")")?;
        }
        // set next_item_is_key to false
        state.get_current_scope().next_item_is_key = false;
        match state.options.formatting {
            Formatting::Indented { .. } => {
                write!(output, ": ")?;
            }
            Formatting::Compact => {
                write!(output, ":")?;
            }
        }
        // prevent redundant comma before value
        state.get_current_scope().skip_comma_for_next_item = true;
    }

    Ok(())
}

/// before scope close (insert scope closing syntax)
fn handle_scope_close(
    state: &mut DecompilerState,
    output: &mut String,
    indentation_levels: usize,
) -> Result<(), DXBParserError> {
    let formatting = state.options.formatting;
    let scope = state.get_current_scope();
    // close only if not outer scope
    if !scope.is_outer_scope {
        state.get_current_scope().write_end(output, &formatting, indentation_levels)?;
    }
    // close scope
    state.close_scope();
    Ok(())
}

/// insert comma syntax before a term (e.g. ",")
/// if is_standalone_key is set to true, no parenthesis are wrapped around the item if it is a key,
/// e.g. for text ("key": "value") the parenthesis are not needed
fn handle_before_item(
    state: &mut DecompilerState,
    output: &mut String,
    is_standalone_key: bool,
    indentation_levels: usize,
) -> Result<(), DXBParserError> {
    let formatted = state.options.formatting;
    let scope = state.get_current_scope();

    // if next_item_is_key, add opening parenthesis
    if !is_standalone_key && scope.next_item_is_key {
        write!(output, "(")?;
    }

    match scope.scope_type {
        (_, true) => {
            // if first is true, set to false
            scope.scope_type.1 = false;
        }
        (ScopeType::Array | ScopeType::Object | ScopeType::Tuple, false)
            if !scope.skip_comma_for_next_item =>
        {
            match formatted {
                Formatting::Indented { indent } => {
                    write!(output, ",\r\n")?;
                    let current_indent = indentation_levels * indent;
                    for _ in 0..current_indent {
                        write!(output, " ")?;
                    }
                }
                Formatting::Compact => {
                    write!(output, ",")?;
                }
            }
        }
        _ => {
            // don't insert comma for default scope
        }
    }

    // reset skip_comma_for_next_item flag
    scope.skip_comma_for_next_item = false;

    Ok(())
}

/// insert operator syntax before an operand (e.g. +, -, etc.)
fn handle_before_operand(
    state: &mut DecompilerState,
    output: &mut String,
) -> Result<(), DXBParserError> {
    if let Some(operator) = state.get_current_scope().active_operator.take() {
        // handle the operator before the operand
        match operator {
            (_, true) => {
                // if first is true, set to false
                state.get_current_scope().active_operator =
                    Some((operator.0.clone(), false));
            }
            (Instruction::Add, false) => {
                write_operator(state, output, "+")?;
                state.get_current_scope().close_scope_after_term = true;
            }
            (Instruction::Subtract, false) => {
                write_operator(state, output, "-")?;
                state.get_current_scope().close_scope_after_term = true;
            }
            (Instruction::Multiply, false) => {
                write_operator(state, output, "*")?;
                state.get_current_scope().close_scope_after_term = true;
            }
            (Instruction::Divide, false) => {
                write_operator(state, output, "/")?;
                state.get_current_scope().close_scope_after_term = true;
            }
            (Instruction::Union, false) => {
                write_operator(state, output, "|")?;
                state.get_current_scope().close_scope_after_term = true;
            }
            (Instruction::RemoteExecution, false) => {
                write_operator(state, output, "::")?;
                state.get_current_scope().close_scope_after_term = false;
            }
            _ => {
                panic!("Invalid operator: {operator:?}");
            }
        }
    }
    Ok(())
}

fn write_operator(
    state: &mut DecompilerState,
    output: &mut String,
    operator: &str,
) -> Result<(), DXBParserError> {
    write!(output, " {operator} ")?;
    Ok(())
}

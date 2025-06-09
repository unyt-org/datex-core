use std::collections::HashMap; // FIXME no-std
use std::collections::HashSet;
use std::fmt::Write;
use std::io::Cursor;
// FIXME no-std

use crate::datex_values::core_values::decimal::utils::decimal_to_string;
use crate::datex_values_old::SlotIdentifier;
use crate::global::protocol_structures::instructions::{
    DecimalData, Float32Data, Float64Data, FloatAsInt16Data,
    FloatAsInt32Data, Instruction, Int16Data, Int32Data, Int64Data, Int8Data,
    ShortTextData, TextData,
};
use crate::parser::body;
use crate::parser::body::ParserError;
use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSetBuilder};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

lazy_static! {
    static ref NEW_LINE: Regex = Regex::new(r"\r\n").unwrap();
    static ref LAST_LINE: Regex = Regex::new(r"   (.)$").unwrap();
    static ref INDENT: String = "\r\n   ".to_string();
    static ref ALPAHNUMERIC_IDENTIFIER: Regex =
        Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_-]*$").unwrap();
}

/**
 * Converts DXB (with or without header) to DATEX Script
 */
pub fn decompile(dxb: &[u8], options: DecompileOptions) -> String {
    todo!();
    /*let mut body = dxb;

    let header_result = DXBHeader::from_bytes(dxb);

    match header_result {
        // dxb with header
        Ok(header) => {
            body = body::extract_body(header, dxb);
        }
        // assume just dxb body
        Err(_) => (),
    }
    return decompile_body(
        ctx.clone(),
        body,
        formatted,
        colorized,
        resolve_slots,
    );*/
}

pub fn decompile_body(
    dxb_body: &[u8],
    options: DecompileOptions,
) -> Result<String, ParserError> {
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

#[derive(Debug, Clone, Default)]
pub struct DecompileOptions {
    pub formatted: bool,
    pub colorized: bool,
    /// display slots with generated variable names
    pub resolve_slots: bool,
    /// TODO
    /// when set to true, the output is generated as compatible as possible with JSON, e.g. by
    /// always adding double quotes around keys
    pub json_compat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ScopeType {
    #[default]
    Default,
    Tuple,
    Array,
    Object,
}

impl ScopeType {
    pub fn write_start(&self, output: &mut String) -> Result<(), ParserError> {
        match self {
            ScopeType::Default => write!(output, "(")?,
            ScopeType::Tuple => write!(output, "(")?,
            ScopeType::Array => write!(output, "[")?,
            ScopeType::Object => write!(output, "{{")?,
        }
        Ok(())
    }
    pub fn write_end(&self, output: &mut String) -> Result<(), ParserError> {
        match self {
            ScopeType::Default => write!(output, ")")?,
            ScopeType::Tuple => write!(output, ")")?,
            ScopeType::Array => write!(output, "]")?,
            ScopeType::Object => write!(output, "}}")?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct ScopeState {
    /// true if this is the outer scope (default scope)
    is_outer_scope: bool,
    active_operator: Option<(Instruction, bool)>,
    scope_type: (ScopeType, bool),
    /// skip inserted comma for next item (already inserted before key)
    skip_comma_for_next_item: bool,
    /// set to true if next item is a key (e.g. in object)
    next_item_is_key: bool,
}

impl ScopeState {
    fn write_start(&self, output: &mut String) -> Result<(), ParserError> {
        self.scope_type.0.write_start(output)
    }
    fn write_end(&self, output: &mut String) -> Result<(), ParserError> {
        self.scope_type.0.write_end(output)
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

    // returns variable name and variable type if initialization
    fn get_variable_name(&mut self, slot: &SlotIdentifier) -> (String, String) {
        // return slot name
        if slot.is_reserved()
            || slot.is_object_slot()
            || !self.options.resolve_slots
        {
            return (slot.as_string(), "".to_string());
        }
        // existing variable
        if self.variables.contains_key(&slot.index) {
            (
                self.variables
                    .get(&slot.index)
                    .unwrap_or(&"?invalid?".to_string())
                    .to_string(),
                "".to_string(),
            )
        }
        // init variable
        else {
            let name = int_to_label(self.current_label);
            self.current_label += 1;
            self.variables.insert(slot.index, name.clone());
            (name, "var".to_string())
        }
    }
}

fn decompile_loop(state: &mut DecompilerState) -> Result<String, ParserError> {
    let mut output = String::new();

    let instruction_iterator = body::iterate_instructions(state.dxb_body);

    for instruction in instruction_iterator {
        let instruction = instruction?;
        info!("decompile instruction: {:?}", instruction);

        match instruction {
            Instruction::Int8(Int8Data(i8)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{i8}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Int16(Int16Data(i16)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{i16}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Int32(Int32Data(i32)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{i32}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Int64(Int64Data(i64)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{i64}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalF32(Float32Data(f32)) => {
                handle_before_term(state, &mut output, true)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(f32, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalF64(Float64Data(f64)) => {
                handle_before_term(state, &mut output, true)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(f64, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                handle_before_term(state, &mut output, true)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(i16 as f32, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                handle_before_term(state, &mut output, true)?;
                write!(
                    output,
                    "{}",
                    decimal_to_string(i32 as f32, state.options.json_compat)
                )?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Decimal(DecimalData(big_decimal)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{big_decimal}")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::ShortText(ShortTextData(text)) => {
                handle_before_term(state, &mut output, true)?;
                let text = escape_text(&text);
                write!(output, "\"{text}\"")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::Text(TextData(text)) => {
                handle_before_term(state, &mut output, true)?;
                let text = escape_text(&text);
                write!(output, "\"{text}\"")?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::True => {
                handle_before_term(state, &mut output, false)?;
                write!(output, "true")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::False => {
                handle_before_term(state, &mut output, false)?;
                write!(output, "false")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::Null => {
                handle_before_term(state, &mut output, false)?;
                write!(output, "null")?;
                handle_after_term(state, &mut output, false)?;
            }
            Instruction::ArrayStart => {
                handle_before_term(state, &mut output, false)?;
                state.new_scope(ScopeType::Array);
                state.get_current_scope().write_start(&mut output)?;
            }
            Instruction::ObjectStart => {
                handle_before_term(state, &mut output, false)?;
                state.new_scope(ScopeType::Object);
                state.get_current_scope().write_start(&mut output)?;
            }
            Instruction::TupleStart => {
                handle_before_term(state, &mut output, true)?;
                state.new_scope(ScopeType::Tuple);
                state.get_current_scope().write_start(&mut output)?;
            }
            Instruction::ScopeStart => {
                handle_before_term(state, &mut output, true)?;
                state.new_scope(ScopeType::Default);
                state.get_current_scope().write_start(&mut output)?;
            }
            Instruction::ScopeEnd => {
                handle_scope_close(state, &mut output)?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::KeyValueShortText(text_data) => {
                handle_before_term(state, &mut output, false)?;
                // prevent redundant comma for value
                state.get_current_scope().skip_comma_for_next_item = true;
                write_text_key(
                    state,
                    &text_data.0,
                    &mut output,
                    state.options.formatted,
                )?;
            }
            Instruction::KeyValueDynamic => {
                handle_before_term(state, &mut output, false)?;
                state.get_current_scope().skip_comma_for_next_item = true;
                state.get_current_scope().next_item_is_key = true;
            }
            Instruction::CloseAndStore => {
                if state.options.formatted {
                    write!(output, ";\r\n")?;
                } else {
                    write!(output, ";")?;
                }
            }

            // operations
            Instruction::Add => {
                state.get_current_scope().active_operator =
                    Some((Instruction::Add, true));
            }
            Instruction::Subtract => {
                state.get_current_scope().active_operator =
                    Some((Instruction::Subtract, true));
            }
            Instruction::Multiply => {
                state.get_current_scope().active_operator =
                    Some((Instruction::Multiply, true));
            }
            Instruction::Divide => {
                state.get_current_scope().active_operator =
                    Some((Instruction::Divide, true));
            }

            _ => {
                write!(output, "{instruction:?}")?;
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
) -> Result<String, ParserError> {
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
    formatted: bool,
) -> Result<(), ParserError> {
    // if text does not just contain a-z, A-Z, 0-9, _, and starts with a-z, A-Z,  _, add quotes
    let text = if !state.options.json_compat
        && ALPAHNUMERIC_IDENTIFIER.is_match(text)
    {
        text.to_string()
    } else {
        format!("\"{}\"", escape_text(text))
    };
    if formatted {
        write!(output, "{text}: ")?;
    } else {
        write!(output, "{text}:")?;
    }
    Ok(())
}

/// insert syntax before a term (e.g. operators, commas, etc.)
/// if is_standalone_key is set to true, no parenthesis are wrapped around the item if it is a key,
/// e.g. for text ("key": "value") the parenthesis are not needed
fn handle_before_term(
    state: &mut DecompilerState,
    output: &mut String,
    is_standalone_key: bool,
) -> Result<(), ParserError> {
    handle_before_operand(state, output)?;
    handle_before_item(state, output, is_standalone_key)?;
    Ok(())
}

/// if is_standalone_key is set to true, no parenthesis are wrapped around the item if it is a key,
/// e.g. for text ("key": "value") the parenthesis are not needed
fn handle_after_term(
    state: &mut DecompilerState,
    output: &mut String,
    is_standalone_key: bool,
) -> Result<(), ParserError> {
    // next_item_is_key
    if state.get_current_scope().next_item_is_key {
        if !is_standalone_key {
            write!(output, ")")?;
        }
        // set next_item_is_key to false
        state.get_current_scope().next_item_is_key = false;
        if state.options.formatted {
            write!(output, ": ")?;
        } else {
            write!(output, ":")?;
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
) -> Result<(), ParserError> {
    let scope = state.get_current_scope();
    // close only if not outer scope
    if !scope.is_outer_scope {
        state.get_current_scope().write_end(output)?;
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
) -> Result<(), ParserError> {
    let formatted = state.options.formatted;
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
            if formatted {
                write!(output, ", ")?;
            } else {
                write!(output, ",")?;
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
) -> Result<(), ParserError> {
    if let Some(operator) = &state.get_current_scope().active_operator {
        // handle the operator before the operand
        match operator {
            (_, true) => {
                // if first is true, set to false
                state.get_current_scope().active_operator =
                    Some((operator.0.clone(), false));
            }
            (Instruction::Add, false) => {
                write_operator(state, output, "+")?;
            }
            (Instruction::Subtract, false) => {
                write_operator(state, output, "-")?;
            }
            (Instruction::Multiply, false) => {
                write_operator(state, output, "*")?;
            }
            (Instruction::Divide, false) => {
                write_operator(state, output, "/")?;
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
) -> Result<(), ParserError> {
    write!(output, " {operator} ")?;
    Ok(())
}

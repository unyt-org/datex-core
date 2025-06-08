use std::fmt::Write;
use std::collections::HashMap; // FIXME no-std
use std::collections::HashSet;
use std::io::Cursor;
// FIXME no-std

use crate::datex_values_old::SlotIdentifier;
use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSetBuilder};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use crate::datex_values::core_values::decimal::decimal_to_string;
use crate::global::protocol_structures::instructions::{BigDecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, Instruction, Int16Data, Int32Data, Int64Data, Int8Data, ShortTextData, TextData};
use crate::parser::body;
use crate::parser::body::ParserError;

lazy_static! {
    static ref NEW_LINE: Regex = Regex::new(r"\r\n").unwrap();
    static ref LAST_LINE: Regex = Regex::new(r"   (.)$").unwrap();
    static ref INDENT: String = "\r\n   ".to_string();
    static ref ALPAHNUMERIC_IDENTIFIER: Regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_-]*$").unwrap();
}

/**
 * Converts DXB (with or without header) to DATEX Script
 */
pub fn decompile(
    dxb: &[u8],
    options: DecompileOptions,
) -> String {
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

        scopes: vec![
            ScopeState {
                scope_type: (ScopeType::default(), true),
                ..ScopeState::default()
            }
        ],

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
    pub json_compat: bool
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
        if slot.is_reserved() || slot.is_object_slot() || !self.options.resolve_slots {
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

    let instruction_iterator = body::iterate_instructions(
        state.dxb_body,
    );

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
                write!(output, "{}", decimal_to_string(f32, state.options.json_compat))?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalF64(Float64Data(f64)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{}", decimal_to_string(f64, state.options.json_compat))?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{}", decimal_to_string(i16 as f32, state.options.json_compat))?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                handle_before_term(state, &mut output, true)?;
                write!(output, "{}", decimal_to_string(i32 as f32, state.options.json_compat))?;
                handle_after_term(state, &mut output, true)?;
            }
            Instruction::DecimalBig(BigDecimalData(big_decimal)) => {
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
                write_text_key(&text_data.0, &mut output, state.options.formatted)?;
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
                state.get_current_scope().active_operator = Some((Instruction::Add, true));
            }
            Instruction::Subtract => {
                state.get_current_scope().active_operator = Some((Instruction::Subtract, true));
            }
            Instruction::Multiply => {
                state.get_current_scope().active_operator = Some((Instruction::Multiply, true));
            }
            Instruction::Divide => {
                state.get_current_scope().active_operator = Some((Instruction::Divide, true));
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

pub fn apply_syntax_highlighting(datex_script: String) -> Result<String, ParserError> {
    let mut output = String::new();

    // load datex syntax + custom theme
    static DATEX_SCRIPT_DEF: &str = include_str!("../../datex-language/datex.tmbundle/Syntaxes/datex.sublime-text");
    static DATEX_THEME_DEF: &str = include_str!("../../datex-language/themes/datex-dark.tmTheme");
    let mut builder = SyntaxSetBuilder::new();
    let syntax = SyntaxDefinition::load_from_str(
        DATEX_SCRIPT_DEF,
        true,
        None
    ).expect("Failed to load syntax definition");
    builder.add(syntax);
    let theme: Theme = ThemeSet::load_from_reader(&mut Cursor::new(DATEX_THEME_DEF))
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
    text
        .replace('\\', r#"\\"#)
        .replace('"', r#"\""#)
        .replace('\u{0008}', r#"\b"#)
        .replace('\u{000c}', r#"\f"#)
        .replace('\r', r#"\r"#)
        .replace('\t', r#"\t"#)
        .replace('\u{000b}', r#"\v"#)
        .replace('\n', r#"\n"#)
}

fn write_text_key(text: &str, output: &mut String, formatted: bool) -> Result<(), ParserError> {
    // if text does not just contain a-z, A-Z, 0-9, _, and starts with a-z, A-Z,  _, add quotes
    let text = if ALPAHNUMERIC_IDENTIFIER.is_match(text) {
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
fn handle_before_term(state: &mut DecompilerState, output: &mut String, is_standalone_key: bool) -> Result<(), ParserError> {
    handle_before_operand(state, output)?;
    handle_before_item(state, output, is_standalone_key)?;
    Ok(())
}

/// if is_standalone_key is set to true, no parenthesis are wrapped around the item if it is a key,
/// e.g. for text ("key": "value") the parenthesis are not needed
fn handle_after_term(
    state: &mut DecompilerState,
    output: &mut String,
    is_standalone_key: bool
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
fn handle_before_item(state: &mut DecompilerState, output: &mut String, is_standalone_key: bool) -> Result<(), ParserError> {
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
        (ScopeType::Array | ScopeType::Object | ScopeType::Tuple, false) if !scope.skip_comma_for_next_item => {
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
fn handle_before_operand(state: &mut DecompilerState, output: &mut String) -> Result<(), ParserError> {
    if let Some(operator) = &state.get_current_scope().active_operator {
        // handle the operator before the operand
        match operator {
            (_, true) => {
                // if first is true, set to false
                state.get_current_scope().active_operator = Some((operator.0.clone(), false));
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

/**
const text/str = "text";  1 + 2 1 "asdfasdf"
const x = integer/u8;

*/

fn write_operator(state: &mut DecompilerState, output: &mut String, operator: &str) -> Result<(), ParserError> {
    write!(output, " {operator} ")?;
    Ok(())
}

//
// fn decompile_loop(state: &mut DecompilerGlobalState) -> Result<String, ParserError> {
//     let mut out: String = "".to_string();
//
//     // let logger = Logger::new_for_development(&state.ctx, "Decompiler");
//
//     let instruction_iterator = body::iterate_instructions(
//         state.dxb_body,
//         state.index,
//         state.is_end_instruction,
//     );
//
//     // flags - initial values
//     let mut open_element_comma = false;
//     let mut last_was_value = false;
//     let mut last_was_property_access = false;
//     let mut is_indexed_element = false;
//
//     let mut next_assign_action: Option<u8> = None;
//     let mut connective_size_stack: Vec<usize> = vec![];
//     let mut connective_type_stack: Vec<BinaryCode> = vec![];
//
//     for instruction in instruction_iterator {
//         let instruction = instruction?;
//         let code = instruction.code;
//         info!("decompile instruction: {:?}", instruction);
//
//         // is element instruction (in arrays, tuples, ..)
//         let is_new_element = match code {
//             BinaryCode::ELEMENT => true,
//             BinaryCode::ELEMENT_WITH_KEY => true,
//             BinaryCode::ELEMENT_WITH_DYNAMIC_KEY => true,
//             BinaryCode::ELEMENT_WITH_INT_KEY => true,
//             BinaryCode::INTERNAL_OBJECT_SLOT => true,
//             _ => false,
//         };
//
//         // closing array, object, ...
//         let is_closing = match code {
//             BinaryCode::CLOSE_AND_STORE => true,
//             BinaryCode::SCOPE_END => true,
//             BinaryCode::ARRAY_END => true,
//             BinaryCode::OBJECT_END => true,
//             BinaryCode::TUPLE_END => true,
//             _ => false,
//         };
//
//         // binary codes around which there is no space required
//         let no_space_around = match code {
//             BinaryCode::CLOSE_AND_STORE => true,
//             BinaryCode::CHILD_ACTION => true,
//             BinaryCode::CHILD_GET => true,
//             BinaryCode::CHILD_GET_REF => true,
//             BinaryCode::CHILD_SET => true,
//             BinaryCode::CHILD_SET_REFERENCE => true,
//             _ => false,
//         };
//
//         let add_comma = open_element_comma && is_new_element; // comma still has to be closed, possible when the next code starts a new element
//
//         // space between
//         if state.formatted
//             && last_was_value
//             && !add_comma
//             && !no_space_around
//             && !is_indexed_element
//             && !is_closing
//         {
//             out += " ";
//         }
//         last_was_value = true;
//         is_indexed_element = false; // reset
//
//         // check flags:
//         // comma
//         if add_comma {
//             open_element_comma = false;
//             if state.colorized {
//                 out += &Color::DEFAULT.as_ansi_rgb();
//             } // light grey color for property keys
//             out += if state.formatted { ",\r\n" } else { "," }
//         }
//
//         let has_slot = instruction.slot.is_some();
//         let slot = instruction.slot.unwrap_or_default();
//
//         let has_primitive_value = instruction.value.is_some();
//         let primitive_value = instruction.value.unwrap_or_default();
//         let mut custom_primitive_color = false;
//
//         // slot to variable mapping
//         let variable_info = if has_slot {
//             state.get_variable_name(&slot)
//         } else {
//             ("".to_string(), "".to_string())
//         };
//         let variable_name = variable_info.0;
//         let variable_prefix = variable_info.1;
//
//         // coloring
//         if state.colorized {
//             // handle property key strings
//             if last_was_property_access
//                 && (code == BinaryCode::TEXT || code == BinaryCode::SHORT_TEXT)
//                 // && primitive_value.can_omit_quotes()
//             {
//                 out += &get_code_color(&BinaryCode::ELEMENT_WITH_KEY)
//                     .as_ansi_rgb();
//             // light grey color for property keys
//             }
//             // normal coloring
//             else if code != BinaryCode::CLOSE_AND_STORE {
//                 // color is added later for CLOSE_AND_STORE
//                 let color = get_code_color(&code);
//                 if color == Color::_UNKNOWN && has_primitive_value {
//                     custom_primitive_color = true;
//                 } else {
//                     out += &color.as_ansi_rgb();
//                 }
//             }
//         }
//
//         // token to string
//
//         match code {
//             // slot based
//             BinaryCode::INTERNAL_VAR => out += &variable_name.to_string(),
//             // only for backwards compatibility
//             BinaryCode::LABEL => out += &format!("$_{variable_name}"),
//             BinaryCode::SET_INTERNAL_VAR => {
//                 if state.colorized {
//                     out += &Color::RESERVED.as_ansi_rgb();
//                 }
//                 out += &variable_prefix;
//                 if !variable_prefix.is_empty() {
//                     out += " "
//                 };
//                 if state.colorized {
//                     out += &get_code_color(&code).as_ansi_rgb();
//                 }
//                 out += &variable_name;
//                 if state.colorized {
//                     out += &Color::DEFAULT.as_ansi_rgb();
//                 }
//                 out += " = ";
//             }
//             BinaryCode::INIT_INTERNAL_VAR => {
//                 if state.colorized {
//                     out += &Color::RESERVED.as_ansi_rgb();
//                 }
//                 out += &variable_prefix;
//                 if !variable_prefix.is_empty() {
//                     out += " "
//                 };
//                 if state.colorized {
//                     out += &get_code_color(&code).as_ansi_rgb();
//                 }
//                 out += &variable_name;
//                 if state.colorized {
//                     out += &Color::DEFAULT.as_ansi_rgb();
//                 }
//                 out += " := ";
//             }
//             BinaryCode::SET_INTERNAL_VAR_REFERENCE => {
//                 if state.colorized {
//                     out += &Color::RESERVED.as_ansi_rgb();
//                 }
//                 out += &variable_prefix;
//                 if !variable_prefix.is_empty() {
//                     out += " "
//                 };
//                 if state.colorized {
//                     out += &get_code_color(&code).as_ansi_rgb();
//                 }
//                 out += &variable_name;
//                 if state.colorized {
//                     out += &Color::DEFAULT.as_ansi_rgb();
//                 }
//                 out += " $= ";
//             }
//
//             // pointer
//             BinaryCode::INIT_POINTER => {
//                 if state.colorized {
//                     out += &Color::RESERVED.as_ansi_rgb();
//                 }
//                 out += &instruction.value.unwrap().to_string();
//                 if state.colorized {
//                     out += &Color::DEFAULT.as_ansi_rgb();
//                 }
//                 out += " := ";
//             }
//             BinaryCode::SET_POINTER => {
//                 if state.colorized {
//                     out += &Color::RESERVED.as_ansi_rgb();
//                 }
//                 out += &instruction.value.unwrap().to_string();
//                 if state.colorized {
//                     out += &Color::DEFAULT.as_ansi_rgb();
//                 }
//                 out += " =";
//             }
//
//             // assign actions (override primitive value default behaviour)
//             BinaryCode::CHILD_ACTION => {
//                 out +=
//                     &get_code_token(&BinaryCode::CHILD_ACTION, state.formatted)
//             }
//
//             // special primitive value formatting
//             BinaryCode::ELEMENT_WITH_KEY => {
//                 out += &format!("{}:", primitive_value.to_key_string())
//             }
//             BinaryCode::ELEMENT_WITH_INT_KEY => {
//                 out += &format!("{}:", primitive_value.to_key_string())
//             }
//             BinaryCode::INTERNAL_OBJECT_SLOT => {
//                 out += &format!(
//                     "{}:",
//                     SlotIdentifier::new(
//                         primitive_value.get_as_unsigned_integer() as u16
//                     )
//                 )
//             }
//
//             // resolve relativ path, path is stored in text primitive
//             BinaryCode::RESOLVE_RELATIVE_PATH => {
//                 out += primitive_value.get_as_text()
//             }
//
//             // indexed element without key
//             BinaryCode::ELEMENT => {
//                 is_indexed_element = true; // don't add whitespace in front of next value for correct indentation
//             }
//
//             // logical connectives
//             BinaryCode::CONJUNCTION => {
//                 out += "(";
//                 connective_type_stack.push(BinaryCode::CONJUNCTION);
//                 connective_size_stack
//                     .push(primitive_value.get_as_unsigned_integer());
//             }
//             BinaryCode::DISJUNCTION => {
//                 out += "(";
//                 connective_type_stack.push(BinaryCode::DISJUNCTION);
//                 connective_size_stack
//                     .push(primitive_value.get_as_unsigned_integer());
//             }
//
//             // jmp
//             BinaryCode::JMP => {
//                 let label = state.get_insert_label(
//                     primitive_value.get_as_unsigned_integer(),
//                 );
//                 out += &format!("jmp {label}");
//                 if state.colorized {
//                     out += &Color::DEFAULT.as_ansi_rgb();
//                 }
//                 out += ";";
//             }
//             BinaryCode::JTR => {
//                 let label = state.get_insert_label(
//                     primitive_value.get_as_unsigned_integer(),
//                 );
//                 out += &format!("jtr {label}")
//             }
//             BinaryCode::JFA => {
//                 let label = state.get_insert_label(
//                     primitive_value.get_as_unsigned_integer(),
//                 );
//                 out += &format!("jfa {label}")
//             }
//
//             // scope
//             BinaryCode::SCOPE_BLOCK_START => {
//                 let scope = &mut decompile_body(
//                     &primitive_value.get_as_buffer(),
//                     state.formatted,
//                     state.colorized,
//                     state.resolve_slots,
//                 )?;
//
//                 // multi line scope TODO: check multiline (problem cannot check scope.contains(";"), because escape codes can contain ";")
//                 if true {
//                     *scope += ")";
//                     out += "(";
//                     // ----------
//                     if state.formatted {
//                         out += &INDENT;
//                         out += &NEW_LINE.replace_all(
//                             // add spaces to every new line
//                             scope,
//                             &INDENT.to_string(),
//                         );
//                     };
//                     // ----------
//                 } else {
//                     scope.pop(); // remove last character (;)
//                     scope.pop();
//                     scope.pop();
//                     out += scope;
//                 }
//             }
//
//             BinaryCode::CLOSE_AND_STORE => {
//                 // newline+spaces before, remove, add ';' and add newline afterwards
//                 let empty: &[_] = &['\r', '\n', ' '];
//                 out = out.trim_end_matches(empty).to_string();
//                 if state.colorized {
//                     out += &get_code_color(&code).as_ansi_rgb()
//                 }
//                 out += &get_code_token(&code, state.formatted);
//                 // newline if not end of file
//                 if state.formatted && state.index.get() < state.dxb_body.len() {
//                     out += "\r\n";
//                 }
//             }
//
//             _ => {
//                 // primitive value default
//                 if has_primitive_value {
//                     if last_was_property_access {
//                         out += &primitive_value.to_key_string()
//                     } else if custom_primitive_color {
//                         out += &primitive_value.to_string_colorized()
//                     } else {
//                         out += &Value::to_string(&primitive_value)
//                     }
//                 }
//                 // complex value
//                 else if instruction.value.is_some() {
//                     out += &instruction.value.unwrap().to_string();
//                 }
//                 // fallback if no string representation possible [hex code]
//                 else {
//                     out += &get_code_token(&code, state.formatted)
//                 }
//             }
//         }
//
//         // enter new subscope - continue at index?
//         if instruction.subscope_continue {
//             let inner = Cow::from(decompile_loop(state)?);
//             let is_empty = inner.is_empty();
//             let newline_count = inner.chars().filter(|c| *c == '\n').count();
//
//             // only if content inside brackets, and multiple lines
//             if state.formatted && !is_empty && newline_count > 0 {
//                 out += &INDENT;
//                 out += &NEW_LINE.replace_all(
//                     // add spaces to every new line
//                     &inner,
//                     &INDENT.to_string(),
//                 );
//                 out += "\r\n";
//             }
//             // no content inside brackets or single line
//             else {
//                 out += NEW_LINE.replace_all(&inner, "").trim_end(); // remove remaining new line + spaces in last line
//             }
//         }
//
//         // after value insert : finish assign action?
//         if next_assign_action.is_some() {
//             // coloring
//             if state.colorized {
//                 out += &Color::DEFAULT.as_ansi_rgb();
//             }
//             // +=, -=, ...
//             out += " ";
//             let assign_type = next_assign_action.unwrap();
//
//             match assign_type {
//                 1 => out += "$",
//                 2 => out += "",
//                 _ => {
//                     out += &get_code_token(
//                         &BinaryCode::try_from(assign_type)
//                             .expect("enum conversion error"),
//                         false,
//                     )
//                 }
//             }
//             out += "= ";
//             last_was_value = false; // no additional space afterwards
//             next_assign_action = None; // reset
//         }
//
//         // check for new assign actions
//         match code {
//             BinaryCode::CHILD_ACTION => {
//                 next_assign_action =
//                     Some(primitive_value.get_as_integer() as u8)
//             }
//             BinaryCode::CHILD_SET_REFERENCE => next_assign_action = Some(1),
//             BinaryCode::CHILD_SET => next_assign_action = Some(2),
//             _ => (),
//         }
//
//         // reset flags
//         last_was_property_access = false;
//
//         // set flags
//         if is_new_element {
//             open_element_comma = true
//         } // remember to add comma after element
//
//         // ) ] } end
//         if is_closing {
//             // open_element_comma = false; // no more commas required
//             last_was_value = false; // no space afterwards
//         }
//
//         if no_space_around {
//             last_was_value = false // no space afterwards
//         }
//
//         match code {
//             BinaryCode::SET_INTERNAL_VAR => last_was_value = false, // no space afterwards
//             BinaryCode::SET_INTERNAL_VAR_REFERENCE => last_was_value = false, // no space afterwards
//             BinaryCode::INIT_INTERNAL_VAR => last_was_value = false, // no space afterwards
//             BinaryCode::INIT_POINTER => last_was_value = false, // no space afterwards
//             BinaryCode::NOT => last_was_value = false, // no space afterwards
//             BinaryCode::CHILD_GET => last_was_property_access = true, // enable property key formatting for next
//             BinaryCode::CHILD_GET_REF => last_was_property_access = true, // enable property key formatting for next
//             BinaryCode::CHILD_ACTION => last_was_property_access = true, // enable property key formatting for next
//             BinaryCode::CHILD_SET => last_was_property_access = true, // enable property key formatting for next
//             BinaryCode::CHILD_SET_REFERENCE => last_was_property_access = true, // enable property key formatting for next
//
//             BinaryCode::CONJUNCTION => last_was_value = false, // no space afterwards
//             BinaryCode::DISJUNCTION => last_was_value = false, // no space afterwards
//
//             _ => (),
//         }
//
//         // insert label
//         for label in &mut state.labels {
//             // only add if at right index and not yet inserted
//             if *label.0 == state.index.get()
//                 && !state.inserted_labels.contains(label.0)
//             {
//                 if state.colorized {
//                     out += &Color::RESERVED.as_ansi_rgb();
//                 }
//                 out += "\r\nlbl ";
//                 out += label.1;
//                 if state.colorized {
//                     out += &Color::DEFAULT.as_ansi_rgb();
//                 }
//                 out += ";";
//                 state.inserted_labels.insert(*label.0);
//             }
//         }
//
//         // TODO: improve this, last_was_value and stack behaviour is not correct all the time.
//         // This tries to reconstruct the runtime behaviour of inserting values to the stack, which fails e.g for function calls and many other usecases that are not
//         // handled in the decompiler - only permanent 100% fix would be to evaluate the conjunction/disjunction in the runtime and stringify the resulting value, but this
//         // is a big overhead for the decompiler and also might create unintended sideffects...
//
//         // update connective_size and add &/| syntax
//         while last_was_value && !connective_size_stack.is_empty() {
//             let len = connective_size_stack.len() - 1;
//             connective_size_stack[len] -= 1;
//
//             if state.colorized {
//                 out += &Color::DEFAULT.as_ansi_rgb()
//             };
//
//             // connective_size_stack finished
//             if connective_size_stack[len] == 0 {
//                 connective_size_stack.pop();
//                 connective_type_stack.pop();
//                 out += ")";
//                 // possible new loop iteration for next element in stack
//             }
//             // add new connective element
//             else {
//                 out += if connective_type_stack[connective_type_stack.len() - 1]
//                     == BinaryCode::CONJUNCTION
//                 {
//                     " &"
//                 } else {
//                     " |"
//                 };
//                 break; // no further iteration, still in same stack
//             }
//         }
//     }
//
//     if state.colorized {
//         out += AnsiCodes::RESET
//     };
//
//     Ok(out)
// }

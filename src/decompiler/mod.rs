mod constants;


use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::vec;

use constants::tokens::get_code_token;
use crate::datex_values::SlotIdentifier;
use crate::parser::header::has_dxb_magic_number;
use crate::utils::color::Color;
use crate::utils::logger::LoggerContext;
use lazy_static::lazy_static;
use regex::Regex;
use crate::datex_values::Value;

use crate::global::binary_codes::BinaryCode;
use crate::parser::header;
use crate::parser::body;

use self::constants::tokens::get_code_color;

lazy_static!{
	static ref NEW_LINE:Regex = Regex::new(r"\r\n").unwrap();
	static ref LAST_LINE:Regex = Regex::new(r"   (.)$").unwrap();
	static ref INDENT:String ="\r\n   ".to_string();

}


/**
 * Converts DXB (with or without header) to DATEX Script
 */
pub fn decompile(ctx: &LoggerContext, dxb:&[u8], formatted:bool, colorized:bool, resolve_slots:bool) -> String {

	let mut body = dxb;

	// header?
	if has_dxb_magic_number(dxb) {
		let (header, _body) = header::parse_dxb_header(dxb);
		body = _body;
	}

	return decompile_body(ctx, body, formatted, colorized, resolve_slots);
}


pub fn decompile_body(ctx: &LoggerContext, dxb_body:&[u8], formatted:bool, colorized:bool, resolve_slots:bool) -> String {
	
	let mut initial_state = DecompilerGlobalState {
		ctx,
		dxb_body,
		index: &Cell::from(0),

		formatted, 
		colorized,
		resolve_slots,

		current_label: 0,
		labels: HashMap::new(),
		inserted_labels: HashSet::new(),
		variables: HashMap::new(),
	};

	return decompile_loop(&mut initial_state);
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


struct DecompilerGlobalState<'a> {
	// ctx
	ctx: &'a LoggerContext,

	// dxb
	dxb_body:&'a [u8], 
	index: &'a Cell<usize>,

	// options
	formatted: bool,
	colorized: bool,
	resolve_slots: bool, // display slots with generated variable names

	// state
	current_label: i32,
	labels: HashMap<usize, String>,
	inserted_labels: HashSet<usize>,
	variables: HashMap<u16, String>
}

impl DecompilerGlobalState<'_> {
	fn get_insert_label(&mut self, index:usize) -> String {
		// existing
		if self.labels.contains_key(&index) {
			return self.labels.get(&index).or(Some(&"?invalid?".to_string())).unwrap().to_string();
		}
		// new
		else {
			let name = self.current_label.to_string();
			self.current_label += 1;
			self.labels.insert(index, name.clone());
			return name;
		}
	}


	// returns variable name and variable type if initialization
	fn get_variable_name(&mut self, slot:&SlotIdentifier) -> (String, String) {
		// return slot name
		if slot.is_reserved() || slot.is_object_slot() || !self.resolve_slots {
			return (slot.to_string(), "".to_string());
		}
		// existing variable
		if self.variables.contains_key(&slot.index) {
			return (self.variables.get(&slot.index).or(Some(&"?invalid?".to_string())).unwrap().to_string(), "".to_string())
		}
		// init variable
		else {
			let name = int_to_label(self.current_label);
			self.current_label += 1;
			self.variables.insert(slot.index, name.clone());
			return (name, "var".to_string());
		}
	}
}



fn decompile_loop(state: &mut DecompilerGlobalState) -> String {
	let mut out:String = "".to_string();

	// let logger = Logger::new_for_development(&state.ctx, "Decompiler");

	let instruction_iterator = body::iterate_instructions(state.dxb_body, state.index);

	// flags - initial values
	let mut open_element_comma = false;
	let mut last_was_value = false;
	let mut last_was_property_access = false;
	let mut is_indexed_element = false;

	let mut next_assign_action: Option<u8> = None;
	let mut connective_size_stack: Vec<usize> = vec![];
	let mut connective_type_stack:Vec<BinaryCode> = vec![];

	for instruction in instruction_iterator {
		
		let code = instruction.code;

		// is element instruction (in arrays, tuples, ..)
		let is_new_element =  match code {
			BinaryCode::ELEMENT => true,
			BinaryCode::ELEMENT_WITH_KEY => true,
			BinaryCode::ELEMENT_WITH_DYNAMIC_KEY => true,
			BinaryCode::ELEMENT_WITH_INT_KEY => true,
			BinaryCode::INTERNAL_OBJECT_SLOT => true,
			_ => false
		};

		// closing array, object, ...
		let is_closing = match code {
			BinaryCode::CLOSE_AND_STORE => true,
			BinaryCode::SUBSCOPE_END => true,
			BinaryCode::ARRAY_END => true,
			BinaryCode::OBJECT_END => true,
			BinaryCode::TUPLE_END => true,
			_ => false
		};

		// binary codes around which there is no space required
		let no_space_around = match code {
			BinaryCode::CLOSE_AND_STORE => true,
			BinaryCode::CHILD_ACTION => true,
			BinaryCode::CHILD_GET => true,
			BinaryCode::CHILD_GET_REF => true,
			BinaryCode::CHILD_SET => true,
			BinaryCode::CHILD_SET_REFERENCE => true,
			_ => false
		};

		let add_comma = open_element_comma && is_new_element; // comma still has to be closed, possible when the next code starts a new element

		// space between
		if last_was_value && !add_comma && !no_space_around && !is_indexed_element && !is_closing {
			out += " ";
		}
		last_was_value = true;
		is_indexed_element = false; // reset

		// check flags:
		// comma
		if add_comma {
			open_element_comma = false;
			// no comma after last element
			if code != BinaryCode::ARRAY_END && code != BinaryCode::OBJECT_END && code != BinaryCode::TUPLE_END {
				if state.colorized {out += &Color::DEFAULT.as_ansi_rgb();} // light grey color for property keys
				out += if state.formatted {",\r\n"} else {","}
			}
		}

		
		let has_slot = instruction.slot.is_some();
		let slot = instruction.slot.unwrap_or_default();

		let has_primitive_value = instruction.primitive_value.is_some();
		let primitive_value = instruction.primitive_value.unwrap_or_default();
		let mut custom_primitive_color = false;

		// slot to variable mapping
		let variable_info = if has_slot { state.get_variable_name(&slot)} else {("".to_string(),"".to_string())};
		let variable_name = variable_info.0;
		let variable_prefix = variable_info.1;

		// coloring
		if state.colorized {
			// handle property key strings
			if last_was_property_access && (code == BinaryCode::TEXT || code == BinaryCode::SHORT_TEXT) && primitive_value.can_omit_quotes() {
				out += &get_code_color(&BinaryCode::ELEMENT_WITH_KEY).as_ansi_rgb(); // light grey color for property keys
			}
			// normal coloring
			else if code != BinaryCode::CLOSE_AND_STORE { // color is added later for CLOSE_AND_STORE
				let color = get_code_color(&code);
				if color == Color::_UNKNOWN && has_primitive_value {
					custom_primitive_color = true;
				}
				else {
					out += &color.as_ansi_rgb();
				}
			}
		}


		// token to string

	
		match code {
			// slot based
			BinaryCode::INTERNAL_VAR 			    => out += &format!("{variable_name}"),
			BinaryCode::SET_INTERNAL_VAR => {
				if state.colorized {out += &Color::RESERVED.as_ansi_rgb();}
				out += &variable_prefix;
				if variable_prefix.len()!=0 {out += " "};
				if state.colorized {out += &get_code_color(&code).as_ansi_rgb();}
				out += &variable_name;
				if state.colorized {out += &Color::DEFAULT.as_ansi_rgb();}
				out += " = ";
			},
			BinaryCode::INIT_INTERNAL_VAR => {
				if state.colorized {out += &Color::RESERVED.as_ansi_rgb();}
				out += &variable_prefix;
				if variable_prefix.len()!=0 {out += " "};
				if state.colorized {out += &get_code_color(&code).as_ansi_rgb();}
				out += &variable_name;
				if state.colorized {out += &Color::DEFAULT.as_ansi_rgb();}
				out += " := ";
			},
			BinaryCode::SET_INTERNAL_VAR_REFERENCE 	=> {
				if state.colorized {out += &Color::RESERVED.as_ansi_rgb();}
				out += &variable_prefix;
				if variable_prefix.len()!=0 {out += " "};
				if state.colorized {out += &get_code_color(&code).as_ansi_rgb();}
				out += &variable_name;
				if state.colorized {out += &Color::DEFAULT.as_ansi_rgb();}
				out += " $= ";
			},

			// pointer
			BinaryCode::INIT_POINTER => {
				if state.colorized {out += &Color::RESERVED.as_ansi_rgb();}
				out += &instruction.value.unwrap().to_string();
				if state.colorized {out += &Color::DEFAULT.as_ansi_rgb();}
				out += " := ";
			},

			// assign actions (override primitive value default behaviour)
			BinaryCode::CHILD_ACTION => out += &get_code_token(&BinaryCode::CHILD_ACTION, state.formatted),
		
			// special primitive value formatting
			BinaryCode::ELEMENT_WITH_KEY            => out += &format!("{}:", primitive_value.to_key_string()),
			BinaryCode::ELEMENT_WITH_INT_KEY        => out += &format!("{}:", primitive_value.to_key_string()),             
			BinaryCode::INTERNAL_OBJECT_SLOT        => out += &format!("{}:", SlotIdentifier::new(primitive_value.get_as_unsigned_integer() as u16)),        

			// resolve relativ path, path is stored in text primitive
			BinaryCode::RESOLVE_RELATIVE_PATH        => out += primitive_value.get_as_text(),

			// indexed element without key
			BinaryCode::ELEMENT	=> {
				is_indexed_element = true; // don't add whitespace in front of next value for correct indentation
			},

			// logical connectives
			BinaryCode::CONJUNCTION	=> {
				out += "(";
				connective_type_stack.push(BinaryCode::CONJUNCTION);
				connective_size_stack.push(primitive_value.get_as_unsigned_integer());
			},
			BinaryCode::DISJUNCTION	=> {
				out += "(";
				connective_type_stack.push(BinaryCode::DISJUNCTION);
				connective_size_stack.push(primitive_value.get_as_unsigned_integer());
			},

			// jmp
			BinaryCode::JMP	=> {
				let label = state.get_insert_label(primitive_value.get_as_unsigned_integer());
				out += &format!("jmp {}", label);
				if state.colorized {out += &Color::DEFAULT.as_ansi_rgb();}
				out += ";";
			},
			BinaryCode::JTR	=> {
				let label = state.get_insert_label(primitive_value.get_as_unsigned_integer());
				out += &format!("jtr {}", label)
			},
			BinaryCode::JFA	=> {
				let label = state.get_insert_label(primitive_value.get_as_unsigned_integer());
				out += &format!("jfa {}", label)
			},

			// scope
			BinaryCode::SCOPE_BLOCK_START => {
				let scope = &mut decompile_body(&state.ctx, &primitive_value.get_as_buffer(), state.formatted, state.colorized, state.resolve_slots);
				
				// multi line scope TODO: ceck multiline (problem cannot check scope.contains(";"), because escape codes can contain ";")
				if true {
					*scope += ")";
					out += "(";
					// ----------
					if state.formatted {out += &INDENT};
					out += &LAST_LINE.replace_all(     // remove spaces in last line
						&NEW_LINE.replace_all(   // add spaces to every new line
							&scope, 
							&INDENT.to_string()
						), 
					"$1"); 
					// ----------
				}
				else {
					scope.pop(); // remove last character (;)
					scope.pop();
					scope.pop();
					out += scope;
				}
			},

			BinaryCode::CLOSE_AND_STORE => {
				// newline+spaces before, remove, add ';' and add newline afterwards
				let empty: &[_] = &['\r', '\n', ' '];
				out = out.trim_end_matches(empty).to_string();
				if state.colorized {out += &get_code_color(&code).as_ansi_rgb()}
				out += &get_code_token(&code, state.formatted);
				// newline 
				if state.formatted {out += "\r\n"}
			}

			_ => {
				// primitive value default
				if has_primitive_value {
					if last_was_property_access {out += &primitive_value.to_key_string()}
					else if custom_primitive_color {out += &primitive_value.to_string_colorized()}
					else {out += &Value::to_string(&primitive_value)}
				}
				// complex value
				else if instruction.value.is_some() {
					out += &instruction.value.unwrap().to_string();
				}
				// fallback if no string representation possible [hex code]
				else {
					out += &get_code_token(&code, state.formatted)
				}
			}
		}


		// enter new subscope - continue at index?
		if instruction.subscope_continue {
			let inner = Cow::from(decompile_loop(state));
			let is_empty = inner.len() == 8; // only closing ')', ']', ...
			let newline_count = inner.chars().filter(|c| *c == '\n').count();

			// only if content inside brackets, and multiple lines
			if state.formatted && !is_empty && newline_count>1 {
				out += &INDENT;
				out += &LAST_LINE.replace_all(     // remove spaces in last line
					&NEW_LINE.replace_all(   // add spaces to every new line
						&inner, 
						&INDENT.to_string()
					), 
				"$1").trim_end(); 
			}

			// no content inside brackets or single line
			else {
				out += &NEW_LINE.replace_all(&inner, "").trim_end(); // remove remaining new line + spaces in last line
			}
		}


		// after value insert : finish assign action?
		if next_assign_action.is_some() {
			// coloring
			if state.colorized {
				out += &Color::DEFAULT.as_ansi_rgb();
			}
			// +=, -=, ...
			out += " ";
			let assign_type = next_assign_action.unwrap();

			match assign_type {
				1 => out += "$",
				2 => out += "",
				_ => out += &get_code_token(&BinaryCode::try_from(assign_type).expect("enum conversion error"), false)
			}
			out += "= ";
			last_was_value = false; // no additional space afterwards
			next_assign_action = None; // reset
		}

		// check for new assign actions
		match code {
			BinaryCode::CHILD_ACTION => next_assign_action = Some(primitive_value.get_as_integer() as u8),
			BinaryCode::CHILD_SET_REFERENCE => next_assign_action = Some(1),
			BinaryCode::CHILD_SET => next_assign_action = Some(2),
			_ => ()
		}


		// reset flags
		last_was_property_access = false;

		// set flags
		if is_new_element {open_element_comma = true} // remember to add comma after element

		// ) ] } end
		if is_closing {
			open_element_comma = false; // no more commas required 
			last_was_value = false; // no space afterwards
		} 

		if no_space_around {
			last_was_value = false // no space afterwards
		}

		match code {
			BinaryCode::SET_INTERNAL_VAR => {last_was_value = false}, // no space afterwards
			BinaryCode::SET_INTERNAL_VAR_REFERENCE => {last_was_value = false}, // no space afterwards
			BinaryCode::INIT_INTERNAL_VAR => {last_was_value = false}, // no space afterwards
			BinaryCode::INIT_POINTER => {last_was_value = false}, // no space afterwards
			BinaryCode::NOT => {last_was_value = false}, // no space afterwards
			BinaryCode::CHILD_GET => {last_was_property_access = true}, // enable property key formatting for next
			BinaryCode::CHILD_GET_REF => {last_was_property_access = true}, // enable property key formatting for next
			BinaryCode::CHILD_ACTION => {last_was_property_access = true}, // enable property key formatting for next
			BinaryCode::CHILD_SET => {last_was_property_access = true}, // enable property key formatting for next
			BinaryCode::CHILD_SET_REFERENCE => {last_was_property_access = true}, // enable property key formatting for next
			
			BinaryCode::CONJUNCTION => {last_was_value = false}, // no space afterwards
			BinaryCode::DISJUNCTION => {last_was_value = false}, // no space afterwards

			_ => ()
		}
		


		// insert label
		for label in &mut state.labels {
			// only add if at right index and not yet inserted
			if *label.0 == state.index.get() && !state.inserted_labels.contains(label.0) {
				if state.colorized {out += &Color::RESERVED.as_ansi_rgb();}
				out += "\r\nlbl ";
				out += &label.1;
				if state.colorized {out += &Color::DEFAULT.as_ansi_rgb();}
				out += ";";
				state.inserted_labels.insert(*label.0);
			}
		}

		// TODO: improve this, last_was_value and stack behaviour is not correct all the time.
		// This tries to reconstruct the runtime behaviour of inserting values to the stack, which fails e.g for function calls and many other usecases that are not
		// handled in the decompiler - only permanent 100% fix would be to evaluate the conjunction/disjunction in the runtime and stringify the resulting value, but this
		// is a big overhead for the decompiler and also might create unintended sideffects...

		// update connective_size and add &/| syntax
		while last_was_value && connective_size_stack.len()!=0 {
			let len = connective_size_stack.len()-1;
			connective_size_stack[len] -= 1;

			out += &Color::DEFAULT.as_ansi_rgb();

			// connective_size_stack finished
			if connective_size_stack[len] == 0 {
				connective_size_stack.pop();
				connective_type_stack.pop();
				out += ")";
				// possible new loop iteration for next element in stack
			}
			// add new connective element
			else {
				out += if connective_type_stack[connective_type_stack.len()-1] == BinaryCode::CONJUNCTION {" &"} else {" |"};
				break; // no further iteration, still in same stack
			}
		}
	
	
	}

	return out;
}

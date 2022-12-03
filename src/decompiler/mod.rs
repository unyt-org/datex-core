mod constants;


use std::borrow::Cow;
use std::cell::Cell;
use std::ops::Generator;

use constants::tokens::get_code_token;
use crate::utils::color::Color;
use crate::utils::color::get_code_color;
use crate::utils::logger;
use crate::utils::logger::Logger;
use crate::utils::logger::LoggerContext;
use lazy_static::lazy_static;
use regex::Regex;


use crate::global::binary_codes::BinaryCode;
use crate::parser::header;
use crate::parser::body;

lazy_static!{
	static ref NEW_LINE:Regex = Regex::new(r"\r\n").unwrap();
	static ref LAST_LINE:Regex = Regex::new(r"   (.)$").unwrap();
	static ref INDENT:String ="\r\n   ".to_string();

}


/**
 * Converts DXB (with or without header) to DATEX Script
 */
pub fn decompile(ctx: &LoggerContext, dxb:&[u8], formatted:bool, colorized:bool) -> String {

	// header?
	if dxb[0] == 0x01 && dxb[1] == 0x64 {
		header::parse_dxb_header(dxb);
	}

	return decompile_body(ctx, dxb, formatted, colorized);
}


pub fn decompile_body(ctx: &LoggerContext, dxb_body:&[u8], formatted:bool, colorized:bool) -> String {
	
	return decompile_loop(ctx, dxb_body, &Cell::from(0), formatted, colorized);
}


fn decompile_loop(ctx: &LoggerContext, dxb_body:&[u8], index: &Cell<usize>, formatted:bool, colorized:bool) -> String {
	let mut out:String = "".to_string();

	let logger = Logger::new_for_development(&ctx, "Decompiler");

	let instruction_iterator = body::iterate_instructions(dxb_body, index);

	// flags - initial values
	let mut open_element_comma = false;
	let mut last_was_value = false;
	let mut last_was_property_access = false;
	let mut is_indexed_element = false;

	let mut next_assign_action: Option<u8> = None;

	for instruction in instruction_iterator {
		
		let code = instruction.code;

		// is element instruction (in arrays, tuples, ..)
		let is_new_element =  match code {
			BinaryCode::ELEMENT => true,
			BinaryCode::ELEMENT_WITH_KEY => true,
			BinaryCode::ELEMENT_WITH_DYNAMIC_KEY => true,
			BinaryCode::ELEMENT_WITH_INT_KEY => true,
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
			_ => false
		};

		let add_comma = open_element_comma && is_new_element; // comma still has to be closed, possible when the next code starts a new element

		// space between
		if last_was_value && !add_comma && !no_space_around && !is_indexed_element {
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
				out += &Color::DEFAULT.as_ansi_rgb(); // light grey color for property keys
				out += if formatted {",\r\n"} else {","}
			}
		}
		
		// coloring
		if colorized {
			// handle property key strings
			if last_was_property_access && (code == BinaryCode::TEXT || code == BinaryCode::SHORT_TEXT) {
				out += &get_code_color(&BinaryCode::ELEMENT_WITH_KEY).as_ansi_rgb(); // light grey color for property keys
			}
			// normal coloring
			else {
				out += &get_code_color(&code).as_ansi_rgb();
			}
		}


		// token to string

		let slot = instruction.slot.unwrap_or_default();
		let has_primitive_value = instruction.primitive_value.is_some();
		let primitive_value = instruction.primitive_value.unwrap_or_default();

		match code {
			// slot based
			BinaryCode::INTERNAL_VAR 			    => out += &format!("{slot}"),
			BinaryCode::SET_INTERNAL_VAR 			=> out += &format!("{slot} = "),
			BinaryCode::SET_INTERNAL_VAR_REFERENCE 	=> out += &format!("{slot} $= "),
			BinaryCode::INIT_INTERNAL_VAR 		    => out += &format!("{slot} := "),

			// assign actions (override primitive value default behaviour)
			BinaryCode::CHILD_ACTION => out += &get_code_token(&BinaryCode::CHILD_ACTION, formatted),
		
			// special primitive value formatting
			BinaryCode::ELEMENT_WITH_KEY            => out += &format!("{}:", primitive_value.to_key_string()),
			BinaryCode::ELEMENT_WITH_INT_KEY        => out += &format!("{}:", primitive_value.to_key_string()),             

			// indexed element without key
			BinaryCode::ELEMENT	=> {
				is_indexed_element = true; // don't add whitespace in front of next value for correct indentation
			},

			// scope
			BinaryCode::SCOPE_BLOCK_START => {
				let scope = &mut decompile_body(&ctx, &primitive_value.get_as_buffer(), formatted, colorized);
				
				// multi line scope TODO: ceck multiline (problem cannot check scope.contains(";"), because escape codes can contain ";")
				if true {
					*scope += ")";
					out += "(";
					// ----------
					if formatted {out += &INDENT};
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

			_ => {
				// primitive value default
				if has_primitive_value {
					if last_was_property_access {out += &primitive_value.to_key_string()}
					else {out += &primitive_value.to_string()}
				}
				// complex value
				else if instruction.value.is_some() {
					out += &instruction.value.unwrap().to_string();
				}
				// fallback if no string representation possible [hex code]
				else {
					out += &get_code_token(&code, formatted)
				}
			}
		}


		// enter new subscope - continue at index?
		if instruction.subscope_continue {
			let inner = Cow::from(decompile_loop(ctx, dxb_body, index, formatted, colorized));
			let is_empty = inner.len() == 8; // only closing ')', ']', ...
			
			// only if content inside brackets
			if !is_empty {
				if formatted {out += &INDENT};
				out += &LAST_LINE.replace_all(     // remove spaces in last line
					&NEW_LINE.replace_all(   // add spaces to every new line
						&inner, 
						&INDENT.to_string()
					), 
				"$1"); 
			}

			// no content inside brackets
			else {
				out += &NEW_LINE.replace_all(&inner, ""); // remove remaining new line
			}
		}


		// after value insert : finish assign action?
		if next_assign_action.is_some() {
			// coloring
			if colorized {
				out += &Color::DEFAULT.as_ansi_rgb();
			}
			// +=, -=, ...
			out += " ";
			out += &get_code_token(&BinaryCode::try_from(next_assign_action.unwrap()).expect("enum conversion error"), false);
			out += "= ";
			last_was_value = false; // no additional space afterwards
			next_assign_action = None; // reset
		}

		// check for new assign actions
		match code {
			BinaryCode::CHILD_ACTION => next_assign_action = Some(primitive_value.get_as_integer() as u8),
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
			BinaryCode::CHILD_GET => {last_was_property_access = true}, // enable property key formatting for next
			BinaryCode::CHILD_GET_REF => {last_was_property_access = true}, // enable property key formatting for next
			BinaryCode::CHILD_ACTION => {last_was_property_access = true}, // enable property key formatting for next
			_ => ()
		}
		
	
	}

	return out;
}

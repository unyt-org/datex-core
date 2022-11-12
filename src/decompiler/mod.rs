mod constants;


use std::cell::Cell;
use std::ops::Generator;

use constants::tokens::get_code_token;
use constants::color::get_code_color;
use lazy_static::lazy_static;
use regex::Regex;


use crate::global::binary_codes::BinaryCode;
use crate::parser::body::Instruction;
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
pub fn decompile(dxb:&[u8], formatted:bool, colorized:bool) -> String {

	// header?
	if dxb[0] == 0x01 && dxb[1] == 0x64 {
		header::parse_dxb_header(dxb);
	}

	return decompile_body(dxb, formatted, colorized);
}


pub fn decompile_body(dxb_body:&[u8], formatted:bool, colorized:bool) -> String {
	
	return decompile_loop(dxb_body, &Cell::from(0), formatted, colorized);
}


fn decompile_loop(dxb_body:&[u8], index: &Cell<usize>, formatted:bool, colorized:bool) -> String {
	let mut out:String = "".to_string();

	let instruction_iterator = body::iterate_instructions(dxb_body, index);

	// flags
	let mut element_comma = false;
	let mut element_comma_skip = false;

	for instruction in instruction_iterator {
		
		let code = instruction.code;

		// coloring
		if colorized {
			out += &get_code_color(&code).as_ansi_rgb();
		}

		// check flags:
		// comma
		if element_comma_skip { element_comma_skip = false;} // skip once
		else if element_comma {
			element_comma = false;
			// no comma after last element
			if code != BinaryCode::ARRAY_END && code != BinaryCode::OBJECT_END && code != BinaryCode::TUPLE_END {
				out += if formatted {",\r\n"} else {","}
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

			// special primitive value formatting
			BinaryCode::ELEMENT_WITH_KEY            => out += &format!("{}:", primitive_value.to_key_string()),
			BinaryCode::ELEMENT_WITH_INT_KEY        => out += &format!("{}:", primitive_value.to_key_string()),             

			// implemented flags below
			BinaryCode::ELEMENT	=> (),

			_ => {
				// primitive value default
				if has_primitive_value {
					out += &primitive_value.to_string();
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
			if formatted {out += &INDENT};
			out += &LAST_LINE.replace_all(&NEW_LINE.replace_all( // remove spaces in last line
				&decompile_loop(dxb_body, index, formatted, colorized),  // add spaces to every new line
				&INDENT.to_string()
			), "$1");
		}

		// set flags
		match code {
			BinaryCode::ELEMENT	=> {element_comma = true; element_comma_skip = true;}, // skip next element, then insert
			BinaryCode::ELEMENT_WITH_KEY	=> {element_comma = true; element_comma_skip = true;}, // skip next element, then insert
			BinaryCode::ELEMENT_WITH_INT_KEY	=> {element_comma = true; element_comma_skip = true;}, // skip next element, then insert
			BinaryCode::ELEMENT_WITH_DYNAMIC_KEY	=> {element_comma = true; element_comma_skip = true;}, // skip next element, then insert

			_ => ()
		}
		
	
	}

	return out;
}
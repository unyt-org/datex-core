mod constants;



use constants::tokens::get_code_token;
use constants::color::get_code_color;

use crate::Logger;

use crate::datex_values::SlotIdentifier;
use crate::global::binary_codes::BinaryCode;
use crate::parser::header;
use crate::parser::body;

/**
 * Converts DXB (with or without header) to DATEX Script
 */
pub fn decompile(dxb:&[u8], formatted:bool, colorized:bool) -> String {
	let logger:Logger = Logger::new("DATEX WASM Decompiler");
	logger.info(&format!("Decompling ..."));

	// header?
	if dxb[0] == 0x01 && dxb[1] == 0x64 {
		logger.info("has header");
		header::parse_dxb_header(dxb);
	}

	return decompile_body(dxb, formatted, colorized);
}

pub fn decompile_body(dxb:&[u8], formatted:bool, colorized:bool) -> String {
	let _logger:Logger = Logger::new("DATEX WASM Decompiler");

	let mut out:String = "".to_string();

	for x in body::parse_loop(dxb) {
		
		let code = x.code;

		// coloring
		if colorized {
			out += &get_code_color(&code).as_ansi_rgb();
		}

		// primitive value -> to string
		if x.primitive_value.is_some() {
			out += &x.primitive_value.unwrap().to_string();
		}
		
		// other token
		else {

			let slot = x.slot.unwrap_or_default();

			match code {
				BinaryCode::INTERNAL_VAR 			    => out += &format!("{slot}"),
				BinaryCode::SET_INTERNAL_VAR 			=> out += &format!("{slot} = "),
				BinaryCode::SET_INTERNAL_VAR_REFERENCE 	=> out += &format!("{slot} $= "),
				BinaryCode::INIT_INTERNAL_VAR 		    => out += &format!("{slot} := "),

				_ => out += &get_code_token(&code)
			}

			
		}

		// formatting (new lines, indent)
		if formatted {
			if code == BinaryCode::CLOSE_AND_STORE {
				out += "\r\n";
			}
		}
		
	}

	return out;
}
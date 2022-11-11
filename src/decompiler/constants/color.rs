use crate::global::binary_codes::BinaryCode;



struct AnsiCodes {}
impl AnsiCodes {
    const COLOR_DEFAULT: &'static str = "\x1b[39m";
}

pub enum Color {
	TEXT,
	NUMBER,
	BUFFER,
	PRIMITIVE_CONSTANT,
	DEFAULT,
	RESERVED
}


fn ansi_rgb(r:u8, g:u8, b:u8) -> String {
	return format!("\x1b[38;2;{r};{g};{b}m")
}


impl Color {

    pub fn as_ansi_rgb(&self) -> String {
        match self {
            Color::TEXT => ansi_rgb(183,129,227),
			Color::NUMBER => ansi_rgb(253,139,25),
			Color::PRIMITIVE_CONSTANT => ansi_rgb(219,45,129),
			Color::BUFFER => ansi_rgb(238,95,95),

            Color::RESERVED => ansi_rgb(65,102,238),
			Color::DEFAULT => AnsiCodes::COLOR_DEFAULT.to_string()
        }
    }

	pub fn as_ansi_8_bit(&self) -> &'static str {
        match self {
            _ => ""
        }
    }
}


pub fn get_code_color(code: &BinaryCode) -> Color {
	match code {
		BinaryCode::TEXT => Color::TEXT,
		BinaryCode::SHORT_TEXT => Color::TEXT,

		BinaryCode::BUFFER => Color::BUFFER,

		BinaryCode::INT_8 => Color::NUMBER,
		BinaryCode::INT_16 => Color::NUMBER,
		BinaryCode::INT_32 => Color::NUMBER,
		BinaryCode::INT_64 => Color::NUMBER,
		BinaryCode::FLOAT_64 => Color::NUMBER,
		BinaryCode::FLOAT_AS_INT => Color::NUMBER,

		BinaryCode::TRUE => Color::PRIMITIVE_CONSTANT,
		BinaryCode::FALSE => Color::PRIMITIVE_CONSTANT,
		BinaryCode::NULL => Color::PRIMITIVE_CONSTANT,
		BinaryCode::VOID => Color::PRIMITIVE_CONSTANT,

		BinaryCode::RETURN=> Color::RESERVED,
		BinaryCode::TEMPLATE=> Color::RESERVED,
		BinaryCode::EXTENDS=> Color::RESERVED,
		BinaryCode::IMPLEMENTS=> Color::RESERVED,
		BinaryCode::MATCHES=> Color::RESERVED,
		BinaryCode::DEBUGGER=> Color::RESERVED,
		BinaryCode::JMP=> Color::RESERVED,
		BinaryCode::JTR=> Color::RESERVED,
		BinaryCode::JFA=> Color::RESERVED,
		BinaryCode::COUNT=> Color::RESERVED,
		BinaryCode::ABOUT=> Color::RESERVED,
		BinaryCode::NEW=> Color::RESERVED,
		BinaryCode::DELETE_POINTER=> Color::RESERVED,
		BinaryCode::COPY=> Color::RESERVED,
		BinaryCode::CLONE=> Color::RESERVED,
		BinaryCode::ORIGIN=> Color::RESERVED,
		BinaryCode::SUBSCRIBERS=> Color::RESERVED,
		BinaryCode::PLAIN_SCOPE=> Color::RESERVED,
		BinaryCode::TRANSFORM=> Color::RESERVED,
		BinaryCode::OBSERVE=> Color::RESERVED,
		BinaryCode::RUN=> Color::RESERVED,
		BinaryCode::AWAIT=> Color::RESERVED,
		BinaryCode::MAYBE=> Color::RESERVED,
		BinaryCode::FUNCTION=> Color::RESERVED,
		BinaryCode::ASSERT=> Color::RESERVED,
		BinaryCode::ITERATOR=> Color::RESERVED,
		BinaryCode::NEXT=> Color::RESERVED,
		BinaryCode::FREEZE=> Color::RESERVED,
		BinaryCode::SEAL=> Color::RESERVED,
		BinaryCode::HAS=> Color::RESERVED,
		BinaryCode::KEYS=> Color::RESERVED,
		BinaryCode::GET_TYPE=> Color::RESERVED,
		BinaryCode::GET=> Color::RESERVED,
		BinaryCode::DO=> Color::RESERVED,
		BinaryCode::DEFAULT=> Color::RESERVED,
		BinaryCode::COLLAPSE=> Color::RESERVED,

		_ => Color::DEFAULT
	}
}
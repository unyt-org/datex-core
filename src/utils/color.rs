use crate::global::binary_codes::BinaryCode;



pub struct AnsiCodes {}
impl AnsiCodes {
    pub const COLOR_DEFAULT: &'static str = "\x1b[39m";

	pub const CLEAR: &'static str =       "\x1b[2J"; // clear screen

    pub const RESET: &'static str =       "\x1b[0m";
    pub const BOLD: &'static str =        "\x1b[1m";
    pub const DEFAULT: &'static str =     "\x1b[2m";
    pub const ITALIC: &'static str =      "\x1b[3m";
    pub const UNDERLINE: &'static str =  "\x1b[4m";
    pub const INVERSE: &'static str =     "\x1b[7m";
    pub const HIDDEN: &'static str =      "\x1b[8m";

    pub const RESET_UNDERLINE: &'static str =  "\x1b[24m";
    pub const RESET_INVERSE: &'static str =     "\x1b[27m";

    pub const BLACK: &'static str =       "\x1b[30m";
    pub const RED: &'static str =         "\x1b[31m";
    pub const GREEN: &'static str =       "\x1b[32m";
    pub const YELLOW: &'static str =      "\x1b[33m";
    pub const BLUE: &'static str =        "\x1b[34m";
    pub const MAGENTA: &'static str =     "\x1b[35m";
    pub const CYAN: &'static str =        "\x1b[36m";
    pub const WHITE: &'static str =       "\x1b[37m";
    pub const GREY: &'static str =        "\x1b[90m";

    pub const BG_BLACK: &'static str =    "\x1b[40m";
    pub const BG_RED: &'static str =      "\x1b[41m";
    pub const BG_GREEN: &'static str =    "\x1b[42m";
    pub const BG_YELLOW: &'static str =   "\x1b[43m";
    pub const BG_BLUE: &'static str =     "\x1b[44m";
    pub const BG_MAGENTA: &'static str =  "\x1b[45m";
    pub const BG_CYAN: &'static str =     "\x1b[46m";
    pub const BG_WHITE: &'static str =    "\x1b[47m";
    pub const BG_GREY: &'static str =     "\x1b[100m";
    pub const BG_COLOR_DEFAULT: &'static str =  "\x1b[49m";

}

pub enum Color {

	RED,
    GREEN,
    BLUE,
    YELLOW,
    MAGENTA,
    CYAN,
    BLACK,
    WHITE,
    GREY,


	TEXT,
	NUMBER,
	BUFFER,
	PRIMITIVE_CONSTANT,
	TYPE,

	DEFAULT,
	DEFAULT_LIGHT,
	RESERVED
}


fn ansi_rgb(r:u8, g:u8, b:u8) -> String {
	return format!("\x1b[38;2;{r};{g};{b}m")
}


impl Color {

    pub fn as_ansi_rgb(&self) -> String {
        match self {
			Color::RED => ansi_rgb(234,43,81),
            Color::GREEN => ansi_rgb(30,218,109),
            Color::BLUE => ansi_rgb(6,105,193),
            Color::YELLOW => ansi_rgb(235,182,38),
            Color::MAGENTA => ansi_rgb(196,112,222),
            Color::CYAN => ansi_rgb(79,169,232),
            Color::BLACK => ansi_rgb(5,5,5),
            Color::WHITE => ansi_rgb(250,250,250),
            Color::GREY => ansi_rgb(150,150,150),

            Color::TEXT => ansi_rgb(183,129,227),
			Color::NUMBER => ansi_rgb(253,139,25),
			Color::PRIMITIVE_CONSTANT => ansi_rgb(219,45,129),
			Color::BUFFER => ansi_rgb(238,95,95),
			Color::TYPE => ansi_rgb(50,153,220),

            Color::RESERVED => ansi_rgb(65,102,238),
			Color::DEFAULT => AnsiCodes::COLOR_DEFAULT.to_string(),
			Color::DEFAULT_LIGHT => ansi_rgb(187,187,187)
        }
    }

	pub fn as_ansi_4_bit(&self) -> &'static str {
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

		BinaryCode::TYPE => Color::TYPE,
		BinaryCode::EXTENDED_TYPE => Color::TYPE,

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

		BinaryCode::ELEMENT_WITH_KEY=> Color::DEFAULT_LIGHT,
		BinaryCode::ELEMENT_WITH_INT_KEY=> Color::DEFAULT_LIGHT,

		_ => Color::DEFAULT
	}
}
use crate::{global::binary_codes::BinaryCode, utils::color::Color};

pub fn get_code_token(code: &BinaryCode, formatted:bool) -> String {
	match code {
		BinaryCode::CLOSE_AND_STORE => ";".to_string(),
		
		BinaryCode::ARRAY_START => "[".to_string(),
		BinaryCode::ARRAY_END => if formatted {"\r\n]".to_string()} else {"]".to_string()},
		BinaryCode::OBJECT_START => "{".to_string(),
		BinaryCode::OBJECT_END => if formatted {"\r\n}".to_string()} else {"}".to_string()},
		BinaryCode::TUPLE_START => "(".to_string(),
		BinaryCode::TUPLE_END => if formatted {"\r\n)".to_string()} else {")".to_string()},
		BinaryCode::SUBSCOPE_START => "(".to_string(),
		BinaryCode::SUBSCOPE_END => if formatted {"\r\n)".to_string()} else {")".to_string()},

		BinaryCode::ADD => "+".to_string(),
		BinaryCode::SUBTRACT => "-".to_string(),
		BinaryCode::MULTIPLY => "*".to_string(),
		BinaryCode::DIVIDE => "/".to_string(),
		BinaryCode::POWER => "^".to_string(),
		BinaryCode::MODULO => "%".to_string(),

		BinaryCode::AND => "and".to_string(),
		BinaryCode::OR => "or".to_string(),
		BinaryCode::NOT => "not".to_string(),

		BinaryCode::INCREMENT => "++".to_string(),
		BinaryCode::DECREMENT => "--".to_string(),

		BinaryCode::RUN => "run".to_string(),
		BinaryCode::DO => "do".to_string(),
		BinaryCode::FUNCTION => "function".to_string(),
		BinaryCode::AWAIT => "await".to_string(),
		BinaryCode::COPY => "copy".to_string(),
		BinaryCode::CLONE => "clone".to_string(),
		BinaryCode::CREATE_POINTER => "$$".to_string(),
		BinaryCode::KEYS => "keys".to_string(),
		BinaryCode::GET_TYPE => "type".to_string(),
		BinaryCode::GET => "get".to_string(),
		BinaryCode::YEET => "yeet".to_string(),
		BinaryCode::PLAIN_SCOPE => "scope".to_string(),
		BinaryCode::ASSERT => "assert".to_string(),
		BinaryCode::MATCHES => "matches".to_string(),
		BinaryCode::TRANSFORM => "always".to_string(),
		BinaryCode::RETURN => "return".to_string(),
		BinaryCode::COUNT => "count".to_string(),
		BinaryCode::ITERATOR => "iterator".to_string(),
		BinaryCode::DEBUGGER => "debugger".to_string(),
		BinaryCode::EXTENDS => "extends".to_string(),
		BinaryCode::IMPLEMENTS => "implements".to_string(),
		BinaryCode::NEXT => "next".to_string(),
		BinaryCode::REMOTE => "::".to_string(),

		BinaryCode::GREATER => ">".to_string(),
		BinaryCode::LESS => "<".to_string(),
		BinaryCode::GREATER_EQUAL => ">=".to_string(),
		BinaryCode::LESS_EQUAL => "<=".to_string(),
		BinaryCode::NOT_EQUAL => "!==".to_string(),
		BinaryCode::NOT_EQUAL_VALUE => "!=".to_string(),
		BinaryCode::EQUAL => "===".to_string(),
		BinaryCode::EQUAL_VALUE => "==".to_string(),


		BinaryCode::SYNC => "<==".to_string(),
		BinaryCode::STOP_SYNC => "</=".to_string(),
		BinaryCode::_SYNC_SILENT => "<==:".to_string(),


		BinaryCode::STREAM => "<<".to_string(),
		BinaryCode::STOP_STREAM => "</".to_string(),

		BinaryCode::CHILD_GET => ".".to_string(),
		BinaryCode::CHILD_GET_REF => "->".to_string(),
		BinaryCode::CHILD_ACTION => ".".to_string(),
		BinaryCode::CHILD_SET => ".".to_string(),
		BinaryCode::CHILD_SET_REFERENCE => ".".to_string(),

		_ => format!("⎣{:X}⎤", *code as u8).to_string()
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
		BinaryCode::FLOAT_AS_INT_32 => Color::NUMBER,
		BinaryCode::FLOAT_AS_INT_8 => Color::NUMBER,
		BinaryCode::QUANTITY => Color::_UNKNOWN,
		BinaryCode::TIME => Color::TIME,
		BinaryCode::URL => Color::DEFAULT,

		BinaryCode::TRUE => Color::PrimitiveConstant,
		BinaryCode::FALSE => Color::PrimitiveConstant,
		BinaryCode::NULL => Color::PrimitiveConstant,
		BinaryCode::VOID => Color::PrimitiveConstant,

		BinaryCode::TYPE => Color::TYPE,
		BinaryCode::EXTENDED_TYPE => Color::TYPE,

		BinaryCode::PERSON_ALIAS => Color::_UNKNOWN,
		BinaryCode::PERSON_ALIAS_WILDCARD => Color::_UNKNOWN,
		BinaryCode::INSTITUTION_ALIAS => Color::_UNKNOWN,
		BinaryCode::INSTITUTION_ALIAS_WILDCARD => Color::_UNKNOWN,
		BinaryCode::ENDPOINT => Color::_UNKNOWN,
		BinaryCode::ENDPOINT_WILDCARD => Color::_UNKNOWN,


		BinaryCode::RETURN=> Color::RESERVED,
		BinaryCode::TEMPLATE=> Color::RESERVED,
		BinaryCode::YEET=> Color::RESERVED,
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
		BinaryCode::CREATE_POINTER=> Color::RESERVED,
		BinaryCode::POINTER => Color::RESERVED,

		BinaryCode::ELEMENT_WITH_KEY=> Color::DefaultLight,
		BinaryCode::ELEMENT_WITH_INT_KEY=> Color::DefaultLight,

		_ => Color::DEFAULT
	}
}
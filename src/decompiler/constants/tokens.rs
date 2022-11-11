use crate::global::binary_codes::BinaryCode;

pub fn get_code_token(code: &BinaryCode) -> String {
	match code {
		BinaryCode::CLOSE_AND_STORE => ";".to_string(),
		
		BinaryCode::ARRAY_START => "[".to_string(),
		BinaryCode::ARRAY_END => "]".to_string(),
		BinaryCode::OBJECT_START => "{".to_string(),
		BinaryCode::OBJECT_END => "}".to_string(),
		BinaryCode::TUPLE_START => "(".to_string(),
		BinaryCode::TUPLE_END => ")".to_string(),
		BinaryCode::SUBSCOPE_START => "(".to_string(),
		BinaryCode::SUBSCOPE_END => ")".to_string(),

		BinaryCode::ADD => "+".to_string(),
		BinaryCode::SUBTRACT => "-".to_string(),
		BinaryCode::MULTIPLY => "*".to_string(),
		BinaryCode::DIVIDE => "/".to_string(),
		BinaryCode::POWER => "^".to_string(),
		BinaryCode::MODULO => "%".to_string(),

		BinaryCode::AND => " and ".to_string(),
		BinaryCode::OR => " or ".to_string(),
		BinaryCode::NOT => " no ".to_string(),

		BinaryCode::INCREMENT => "++".to_string(),
		BinaryCode::DECREMENT => "--".to_string(),

		BinaryCode::RUN => "run ".to_string(),
		BinaryCode::AWAIT => "await ".to_string(),
		BinaryCode::COPY => "copy ".to_string(),
		BinaryCode::CLONE => "clone ".to_string(),
		BinaryCode::CREATE_POINTER => "$$ ".to_string(),

		
		_ => format!("[{:X}]", *code as u8).to_string()
	}
}


use crate::compiler::parser::DatexParser;
use crate::compiler::parser::Rule;
use crate::datex_values::Endpoint;
use crate::generator::header::append_dxb_header;
use crate::global::binary_codes::BinaryCode;
use crate::global::dxb_block::DXBBlockType;
use crate::global::dxb_block::DXBHeader;
use crate::global::dxb_block::HeaderFlags;
use crate::global::dxb_block::RoutingInfo;
use crate::utils::buffers::append_f64;
use crate::utils::buffers::append_i16;
use crate::utils::buffers::append_i32;
use crate::utils::buffers::append_i64;
use crate::utils::buffers::append_i8;

use crate::utils::buffers::append_u32;
use crate::utils::buffers::append_u8;

pub mod parser;
use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest::iterators::Pairs;
use regex::Regex;

pub fn compile(datex_script:&str) -> Result<Vec<u8>, pest::error::Error<Rule>> {

	let body = compile_body(datex_script)?;

	let header = DXBHeader {
		version: 2,
		size: 65535,
		signed: true,
		encrypted: true,

		block_type: DXBBlockType::REQUEST,
		scope_id: 22,
		block_index: 1,
		block_increment: 2,
		timestamp: 1234,

		flags: HeaderFlags {
			end_of_scope: true,
			allow_execute: true,
			device_type: 0
		},

		routing: RoutingInfo {
			ttl: 14,
			priority: 40,
			sender: Some(Endpoint::new_person("@theo", Endpoint::ANY_INSTANCE))
		}
	};
	
	Ok(append_dxb_header(&header, &body))
}


struct CompilationScope<'a> {
	index: usize,
	buffer: &'a mut Vec<u8>
}

impl<'a> CompilationScope<'a> {

	const MAX_INT_32:i64 = 2_147_483_647;
    const MIN_INT_32:i64 = -2_147_483_648;

    const MAX_INT_8:i64 = 127;
    const MIN_INT_8:i64 = -128;

    const MAX_INT_16:i64 = 32_767;
    const MIN_INT_16:i64 = -32_768;

    const MAX_UINT_16:i64 = 65_535;

	const INT_8_BYTES: u8 = 1;
	const INT_16_BYTES: u8 = 2;
	const INT_32_BYTES: u8 = 4;
	const INT_64_BYTES: u8 = 8;
	const UINT_8_BYTES: u8 = 1;
	const UINT_16_BYTES: u8 = 2;
	const UINT_32_BYTES: u8 = 4;
	const UINT_64_BYTES: u8 = 8;
	const FLOAT_64_BYTES: u8 = 8;

	// value insert functions
	fn insert_boolean(&mut self, boolean: bool) {
		if boolean {
			self.append_binary_code(BinaryCode::TRUE);
		}
		else {
			self.append_binary_code(BinaryCode::FALSE);
		}
	}

	fn insert_string(&mut self, string: &str) {

		let unescaped_string = self.unescape_string(string);
		
		let bytes = unescaped_string.as_bytes();
		let len = bytes.len();

		if len < 256 {
			self.append_binary_code(BinaryCode::SHORT_TEXT);
			self.append_u8(len as u8);
		}

		else {
			self.append_binary_code(BinaryCode::TEXT);
			self.append_u32(len as u32);
		}

		self.append_buffer(bytes);

	}

	fn unescape_string(&mut self, string: &str) -> String {
		let re = Regex::new(r"\\(.)").unwrap();
		
		// TODO: escape, unicode, hex, octal?
		re.replace_all(
			&string
				.replace("\\b", "\u{0008}")
				.replace("\\f", "\u{000c}")
				.replace("\\r", "\r")
				.replace("\\t", "\t")
				.replace("\\v", "\u{000b}")
				.replace("\\n", "\n"),
		 "$1").into_owned()
	}

	fn insert_float64(&mut self, float64: f64) {
		self.append_binary_code(BinaryCode::FLOAT_64);
		self.append_f64(float64);
	}

	fn insert_int(&mut self, int: i64) {
		if int<=CompilationScope::MAX_INT_8 && int>= CompilationScope::MIN_INT_8 {self.insert_int8(int as i8)}
		else if int<=CompilationScope::MAX_INT_16 && int>=CompilationScope::MIN_INT_16 {self.insert_int16(int as i16)}
		else if int<=CompilationScope::MAX_INT_32 && int>=CompilationScope::MIN_INT_32 {self.insert_int32(int as i32)}
		else {self.insert_int64(int)}
	}

	fn insert_int8(&mut self, int8: i8) {
		self.append_binary_code(BinaryCode::INT_8);
		self.append_i8(int8);
	}
	fn insert_int16(&mut self, int16: i16) {
		self.append_binary_code(BinaryCode::INT_16);
		self.append_i16(int16);
	}
	fn insert_int32(&mut self, int32: i32) {
		self.append_binary_code(BinaryCode::INT_32);
		self.append_i32(int32);
	}
	fn insert_int64(&mut self, int64: i64) {
		self.append_binary_code(BinaryCode::INT_64);
		self.append_i64(int64);
	}


	// buffer functions
	fn append_u8(&mut self, u8: u8) {
		append_u8(self.buffer, u8);
		self.index += CompilationScope::UINT_8_BYTES as usize;
	}
	fn append_u32(&mut self, u32: u32) {
		append_u32(self.buffer, u32);
		self.index += CompilationScope::UINT_32_BYTES as usize;
	}
	fn append_i8(&mut self, i8: i8) {
		append_i8(self.buffer, i8);
		self.index += CompilationScope::INT_8_BYTES as usize;
	}
	fn append_i16(&mut self, i16: i16) {
		append_i16(self.buffer, i16);
		self.index += CompilationScope::INT_16_BYTES as usize;
	}
	fn append_i32(&mut self, i32: i32) {
		append_i32(self.buffer, i32);
		self.index += CompilationScope::INT_32_BYTES as usize;
	}
	fn append_i64(&mut self, i64: i64) {
		append_i64(self.buffer, i64);
		self.index += CompilationScope::INT_64_BYTES as usize;
	}
	fn append_f64(&mut self, f64: f64) {
		append_f64(self.buffer, f64);
		self.index += CompilationScope::FLOAT_64_BYTES as usize;
	}
	fn append_string_utf8(&mut self, string: &str) {
		let bytes = string.as_bytes();
		self.buffer.extend_from_slice(bytes);
		self.index += bytes.len()
	}
	fn append_buffer(&mut self, buffer: &[u8]) {
		self.buffer.extend_from_slice(buffer);
		self.index += buffer.len()
	}

	fn append_binary_code(&mut self, binary_code: BinaryCode) {
		self.append_u8(binary_code as u8);
	}

}

pub fn compile_body(datex_script:&str) -> Result<Vec<u8>, Error<Rule>> {
	let pairs = DatexParser::parse(Rule::datex, datex_script)?;//.next().unwrap();

	// println!("{:}", res);

	let mut buffer = Vec::with_capacity(256);
	let compilation_scope = CompilationScope {buffer: &mut buffer, index: 0};

	parse_statements(compilation_scope, pairs);

	Ok(buffer)
}

fn parse_statements(mut compilation_scope: CompilationScope, pairs: Pairs<'_, Rule>) {

	for pair in pairs {
		match pair.as_rule() {
			Rule::statement => {
				// println!("statement {:#?}", pair);
				for inner in pair.into_inner() {
					parse(&mut compilation_scope, inner)
				}
				compilation_scope.append_binary_code(BinaryCode::CLOSE_AND_STORE);
				// compilation_scope.buffer.push(BinaryCode::STD_TYPE_MAP as u8);
			}
			Rule::EOI => {
				//
			}
			_ => {
				panic!("Invalid rule, expected statement")
			}
		}

	}
}

fn parse(compilation_scope: &mut CompilationScope, pair: Pair<'_, Rule>) {

	let rule = pair.as_rule();
	match rule {
		Rule::integer => {
			let int = pair.as_str().parse::<i64>().unwrap();
			compilation_scope.insert_int(int);
		}
		Rule::decimal => {
			let decimal = pair.as_str().parse::<f64>().unwrap();
			compilation_scope.insert_float64(decimal);
		}
		Rule::string => {
			let string = pair.as_str();
			let inner_string = &string[1..string.len()-1];
			compilation_scope.insert_string(inner_string);
		}
		_ => {
			panic!("Rule not implemented")
		}
	}

}
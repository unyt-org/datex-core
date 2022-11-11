
use crate::Logger;


pub fn compile(datex_script:&str) -> &str {
	let logger:Logger = Logger::new("DATEX WASM Compiler");

	logger.info(&format!("Compiling Script: {datex_script}"));

	"\x01\x64\x01\x00\x00"
}


//let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();

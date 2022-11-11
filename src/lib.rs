#![feature(generator_trait)]
#![feature(generators)]

mod compiler;
mod decompiler;
mod parser;
mod global;
mod utils;
mod datex_values;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


// import Datex.Logger from JavaScript
#[wasm_bindgen(raw_module = "../../../utils/logger.js")]
extern "C" {
    type Logger;

    #[wasm_bindgen(constructor, js_class = Logger)]
    fn new(name:&str) -> Logger;

    #[wasm_bindgen(method)]
    fn success(this: &Logger, message:&str);
    #[wasm_bindgen(method)]
    fn error(this: &Logger, message:&str);
    #[wasm_bindgen(method)]
    fn info(this: &Logger, message:&str);
    #[wasm_bindgen(method)]
    fn warn(this: &Logger, message:&str);
    #[wasm_bindgen(method)]
    fn debug(this: &Logger, message:&str);
}


// export compiler/runtime functions to JavaScript
#[wasm_bindgen]
pub fn init_runtime() {
    let x = Logger::new("DATEX WASM Runtime");
    x.success("initialized")
}


#[wasm_bindgen]
pub fn compile(datex_script:&str) -> String {
    compiler::compile(datex_script).to_string()
}

#[wasm_bindgen]
pub fn decompile(dxb:&[u8], formatted: bool, colorized:bool) -> String {
    return decompiler::decompile(dxb, formatted, colorized);
}
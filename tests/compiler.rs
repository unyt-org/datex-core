use datex_core::{compiler::{compile, parser::{DatexParser, Rule}, compile_body}, decompiler::decompile_body, runtime::Runtime};
use pest::Parser;

#[test]
pub fn compile_literals() {
	compare_with_decompiled("42;");
	compare_with_decompiled("4200000000000;");
	compare_with_decompiled("1.23;");
	compare_with_decompiled("\"Hello World\";");
	compare_with_decompiled("\"#ölz1中文\";");
	compare_with_decompiled("\"\\\\\";");
	// compare_with_decompiled("\"\\\"\";");
}

fn compare_with_decompiled(datex_script: &str) {
	let runtime = Runtime::new();
	let dxb_body = compile_body(&datex_script).unwrap();

	let decompiled = decompile_body(runtime.ctx, &dxb_body, false, false, false);
	let decompiled_color = decompile_body(runtime.ctx, &dxb_body, true, true, true);

	println!("original   : {}", datex_script);
	println!("decompiled : {}", decompiled_color);
	assert_eq!(datex_script, decompiled)
}


#[test]
pub fn compile_raw_tokens() {
	let dxb = DatexParser::parse(Rule::datex, "1;2");

	println!("{:#?}", dxb);
}

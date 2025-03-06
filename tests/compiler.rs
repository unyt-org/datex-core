use datex_core::{
  compiler::{
    compile_body,
    parser::{DatexParser, Rule},
  },
  decompiler::decompile_body,
  runtime::Runtime,
};
use pest::Parser;

#[test]
pub fn compile_literals() {
  compare_compiled_with_decompiled("42;");
  compare_compiled_with_decompiled("4200000000000;");
  compare_compiled_with_decompiled("1.23;");
  compare_compiled_with_decompiled(r#""Hello World";"#);
  compare_compiled_with_decompiled(r#""ölz1中文";"#);
  compare_compiled_with_decompiled(r#""\\";"#);
  compare_compiled_with_decompiled(r#""\\\"";"#);
  compare_compiled_with_decompiled(r#""\n";"#);
  compare_compiled_with_decompiled(r#""\r\n";"#);
  compare_compiled_with_decompiled(r#""\t";"#);
  compare_compiled(
    r#""a
b
c";"#,
    "\"a\\nb\\nc\";",
  );
}

fn compare_compiled_with_decompiled(datex_script: &str) {
  let runtime = Runtime::default();
  let dxb_body = compile_body(&datex_script).unwrap();

  let decompiled =
    decompile_body(runtime.ctx.clone(), &dxb_body, false, false, false);
  let decompiled_color =
    decompile_body(runtime.ctx.clone(), &dxb_body, true, true, true);

  println!("original   : {}", datex_script);
  println!("decompiled : {}", decompiled_color);
  assert_eq!(datex_script, decompiled)
}

fn compare_compiled(datex_script: &str, expected: &str) {
  let runtime = Runtime::default();
  let dxb_body = compile_body(&datex_script).unwrap();

  let decompiled_color =
    decompile_body(runtime.ctx.clone(), &dxb_body, true, true, true);
  let decompiled =
    decompile_body(runtime.ctx.clone(), &dxb_body, false, false, false);

  println!("original   : {}", datex_script);
  println!("expected : {}", expected);
  println!("decompiled : {}", decompiled_color);
  assert_eq!(expected, decompiled)
}

#[test]
pub fn compile_raw_tokens() {
  let dxb = DatexParser::parse(Rule::datex, "1;2");

  println!("{:#?}", dxb);
}

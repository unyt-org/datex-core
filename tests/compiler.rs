use datex_core::{
    compiler::{
        compile_block,
        parser::{DatexParser, Rule},
    },
    decompiler::decompile_body,
};
use log::info;
use pest::Parser;
use datex_core::compiler::bytecode::compile_script;
use datex_core::logger::init_logger;

fn compare_compiled_with_decompiled(datex_script: &str) {
    let dxb_body = compile_script(datex_script).unwrap();

    let decompiled = decompile_body(&dxb_body, false, false, false)
        .unwrap_or_else(|err| panic!("Failed to decompile: {err:?}"));
    // let decompiled_color = decompile_body(&dxb_body, true, true, true)
    //     .unwrap_or_else(|err| panic!("Failed to decompile with color: {err:?}"));

    info!("original   : {datex_script}");
    info!("decompiled : {decompiled}");
    assert_eq!(datex_script, decompiled)
}

fn compare_compiled(datex_script: &str, expected: &str) {
    let dxb_body = compile_script(datex_script).unwrap();

    let decompiled_color = decompile_body(&dxb_body, true, true, true)
        .unwrap_or_else(|err| panic!("Failed to decompile: {err:?}"));
    let decompiled = decompile_body(&dxb_body, false, false, false)
        .unwrap_or_else(|err| panic!("Failed to decompile: {err:?}"));

    info!("original   : {datex_script}");
    info!("expected : {expected}");
    info!("decompiled : {decompiled_color}");
    assert_eq!(expected, decompiled)
}

#[test]
pub fn compile_literals() {
    init_logger();
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


#[test]
pub fn compile_expressions() {
    init_logger();
    compare_compiled_with_decompiled("1 + 2;");
    compare_compiled_with_decompiled("[1, 2]");
}
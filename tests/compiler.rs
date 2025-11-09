use datex_core::compiler::{CompileOptions, compile_script};
use datex_core::decompiler::DecompileOptions;
use datex_core::decompiler::{Formatting, decompile_body};
use datex_core::logger::init_logger_debug;
use log::info;

fn compare_compiled_with_decompiled(datex_script: &str) {
    let (dxb_body, _) =
        compile_script(datex_script, CompileOptions::default()).unwrap();

    let decompiled = decompile_body(&dxb_body, DecompileOptions::default())
        .unwrap_or_else(|err| core::panic!("Failed to decompile: {err:?}"));
    // let decompiled_color = decompile_body(&dxb_body, true, true, true)
    //     .unwrap_or_else(|err| core::panic!("Failed to decompile with color: {err:?}"));

    info!("original   : {datex_script}");
    info!("decompiled : {decompiled}");
    assert_eq!(datex_script, decompiled)
}

fn compare_compiled(datex_script: &str, expected: &str) {
    let (dxb_body, _) =
        compile_script(datex_script, CompileOptions::default()).unwrap();

    let decompiled_color = decompile_body(
        &dxb_body,
        DecompileOptions {
            json_compat: false,
            formatting: Formatting::multiline(),
            colorized: true,
            resolve_slots: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|err| core::panic!("Failed to decompile: {err:?}"));
    let decompiled = decompile_body(&dxb_body, DecompileOptions::default())
        .unwrap_or_else(|err| core::panic!("Failed to decompile: {err:?}"));

    info!("original   : {datex_script}");
    info!("expected : {expected}");
    info!("decompiled : {decompiled_color}");
    assert_eq!(expected, decompiled)
}

#[test]
pub fn compile_literals() {
    init_logger_debug();
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
    compare_compiled_with_decompiled("true");
    compare_compiled_with_decompiled("false");
    compare_compiled_with_decompiled("null");
}

#[test]
pub fn compile_expressions() {
    init_logger_debug();
    compare_compiled_with_decompiled("1 + 2;");
    compare_compiled_with_decompiled("[1,2]");
    // ARR_START 1 2 3 SCOPE_END
    compare_compiled_with_decompiled("[1,2,3 + 4]");
    compare_compiled_with_decompiled("[1,2,[3,4,[5]]];");
    compare_compiled_with_decompiled("[1,2,[3],[4,5],6]");
    compare_compiled_with_decompiled("{a:42,b:\"test\"}");
    compare_compiled_with_decompiled("{a:42,b:\"test\",c:{d:1,e:[2,3]}}");
    compare_compiled_with_decompiled("{\"a b\":42}");
    compare_compiled_with_decompiled("{\"1\":42}");
    compare_compiled_with_decompiled("{(1 + 2):42}");
    // FIXME #280: not working with old decompiler, replace in future
    // compare_compiled_with_decompiled("{(1):42,(1 + 2):42,(true):42}");
}

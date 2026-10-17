use datex_core::{
    compiler::{CompileOptions, compile_script},
    decompiler::{
        DecompileOptions, FormattingOptions,
        dxb_to_source_code::dxb_to_source_code,
    },
    runtime::Runtime,
};
use log::info;

fn compare_compiled_with_decompiled(datex_script: &str) {
    let runtime = Runtime::stub();
    let (dxb, _) =
        compile_script(datex_script, CompileOptions::default(), runtime)
            .unwrap();

    let decompiled = dxb_to_source_code(&dxb, DecompileOptions::compact())
        .unwrap_or_else(|err| core::panic!("Failed to decompile: {err:?}"));

    info!("original   : {datex_script}");
    info!("decompiled : {decompiled}");
    assert_eq!(decompiled, datex_script)
}

fn compare_compiled(datex_script: &str, expected: &str) {
    let runtime = Runtime::stub();

    let (dxb, _) =
        compile_script(datex_script, CompileOptions::default(), runtime)
            .unwrap();

    let decompiled_color = dxb_to_source_code(
        &dxb,
        DecompileOptions {
            formatting_options: FormattingOptions::compact(),
            resolve_slots: true,
        },
    )
    .unwrap_or_else(|err| core::panic!("Failed to decompile: {err:?}"));
    let decompiled = dxb_to_source_code(&dxb, DecompileOptions::default())
        .unwrap_or_else(|err| core::panic!("Failed to decompile: {err:?}"));

    info!("original   : {datex_script}");
    info!("expected : {expected}");
    info!("decompiled : {decompiled_color}");
    assert_eq!(decompiled, expected)
}

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
    compare_compiled_with_decompiled("true");
    compare_compiled_with_decompiled("false");
    compare_compiled_with_decompiled("null");
    compare_compiled_with_decompiled("2026-04-13T18:28:09.415Z");
    compare_compiled_with_decompiled("2026-04-13T18:28:09.000Z");
    compare_compiled_with_decompiled("2026-04-13T18:28:00.000Z");
    compare_compiled_with_decompiled("1970-01-01T00:00:00.000Z");
}

#[test]
pub fn compile_expressions() {
    compare_compiled_with_decompiled("1+2;");
    compare_compiled_with_decompiled("[1,2]");
    compare_compiled_with_decompiled("[1,2,3+4]");
    compare_compiled_with_decompiled("[1,2,[3,4,[5]]];");
    compare_compiled_with_decompiled("[1,2,[3],[4,5],6]");
    compare_compiled_with_decompiled("{a:42,b:\"test\"}");
    compare_compiled_with_decompiled("{a:42,b:\"test\",c:{d:1,e:[2,3]}}");
    compare_compiled_with_decompiled("{\"a b\":42}");
    compare_compiled_with_decompiled("{\"1\":42}");
    compare_compiled_with_decompiled("{(1+2):42}");
    // FIXME #280: not working with old decompiler, replace in future
    // compare_compiled_with_decompiled("{(1):42,(1 + 2):42,(true):42}");
}

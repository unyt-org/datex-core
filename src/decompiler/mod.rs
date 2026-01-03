mod ast_from_bytecode;
mod ast_from_value_container;
mod ast_to_source_code;
mod options;
pub use options::*;

use binrw::io::Cursor;
use core::fmt::Write;

use crate::ast::expressions::{DatexExpression, DatexExpressionData};
use crate::ast::spanned::Spanned;
use crate::decompiler::ast_to_source_code::AstToSourceCodeConverter;

use crate::decompiler::ast_from_bytecode::ast_from_bytecode;
use crate::dxb_parser::body::DXBParserError;
use crate::values::value_container::ValueContainer;
#[cfg(feature = "syntax_highlighting_legacy")]
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::{SyntaxDefinition, SyntaxSetBuilder},
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

/// Decompiles a DXB bytecode body into a human-readable string representation.
pub fn decompile_body(
    dxb_body: &[u8],
    options: DecompileOptions,
) -> Result<String, DXBParserError> {
    let ast = ast_from_bytecode(dxb_body)?;
    Ok(format_ast(ast, options))
}

/// Decompiles a single DATEX value into a human-readable string representation.
pub fn decompile_value(
    value: &ValueContainer,
    options: DecompileOptions,
) -> String {
    let ast = DatexExpressionData::from(value).with_default_span();
    format_ast(ast, options)
}

fn format_ast(ast: DatexExpression, options: DecompileOptions) -> String {
    let colorized = options.formatting_options.colorized;
    let formatter = AstToSourceCodeConverter::new(options.formatting_options);
    // convert AST to source code
    let source = formatter.format(&ast);
    if colorized {
        apply_syntax_highlighting(source).unwrap()
    } else {
        source
    }
}

#[cfg(not(feature = "syntax_highlighting_legacy"))]
pub fn apply_syntax_highlighting(
    datex_script: String,
) -> Result<String, DXBParserError> {
    // skip syntax highlighting
    Ok(datex_script)
}

#[cfg(feature = "syntax_highlighting_legacy")]
pub fn apply_syntax_highlighting(
    datex_script: String,
) -> Result<String, DXBParserError> {
    let mut output = String::new();

    // load datex syntax + custom theme
    static DATEX_SCRIPT_DEF: &str = include_str!(
        "../../datex-language/datex.tmbundle/Syntaxes/datex.sublime-text"
    );
    static DATEX_THEME_DEF: &str =
        include_str!("../../datex-language/themes/datex-dark.tmTheme");
    let mut builder = SyntaxSetBuilder::new();
    let syntax = SyntaxDefinition::load_from_str(DATEX_SCRIPT_DEF, true, None)
        .expect("Failed to load syntax definition");
    builder.add(syntax);
    let theme: Theme =
        ThemeSet::load_from_reader(&mut Cursor::new(DATEX_THEME_DEF))
            .expect("Failed to load theme");

    let ps = builder.build();
    let syntax = ps.find_syntax_by_extension("dx").unwrap();
    let mut h = HighlightLines::new(syntax, &theme);

    for line in LinesWithEndings::from(&datex_script) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        core::write!(output, "{escaped}")?;
    }
    // reset style
    core::write!(output, "\x1b[0m")?;
    Ok(output)
}

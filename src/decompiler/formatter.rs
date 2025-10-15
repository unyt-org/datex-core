// Improved Rust formatter with recursive block indentation, richer template parsing,
// utility functions and clearer method names.

use std::fmt::{self, Display, Write as _};

use crate::ast::tree::{DatexExpression, DatexExpressionData};

/// Formatting mode and options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formatting {
    Compact,
    Multiline { indent: usize }, // indent is number of spaces per level
}
use std::fmt::Write;

#[derive(Debug)]
pub struct FormatConfig {
    pub max_line_length: usize,
    pub indent_size: usize,
    pub use_spaces: bool,
    pub compact: bool,
    pub json_compat: bool,
    pub variant_suffixes: bool,
}
impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            max_line_length: 120,
            indent_size: 4,
            use_spaces: true,
            compact: false,
            json_compat: false,
            variant_suffixes: true,
        }
    }
}

pub struct Formatter<'a> {
    pub config: &'a FormatConfig,
    pub output: String,
    indent_level: usize,
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    /// whether we are permitted to break after this token
    break_after: bool,
    /// atomic tokens must not be split
    atomic: bool,
    /// is whitespace token (we keep them so spacing is preserved)
    is_whitespace: bool,
}

impl Token {
    fn len(&self) -> usize {
        self.text.len()
    }
}

impl<'a> Formatter<'a> {
    pub fn new(config: &'a FormatConfig) -> Self {
        Self {
            config,
            output: String::new(),
            indent_level: 0,
        }
    }

    pub fn increase_indent(&mut self) {
        self.indent_level += 1;
    }
    pub fn decrease_indent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn indent_str(&self) -> String {
        if self.config.use_spaces {
            " ".repeat(self.indent_level * self.config.indent_size)
        } else {
            "\t".repeat(self.indent_level)
        }
    }

    fn continuation_indent_str(&self) -> String {
        let levels = self.indent_level + self.config.continuation_indent_levels;
        if self.config.use_spaces {
            " ".repeat(levels * self.config.indent_size)
        } else {
            "\t".repeat(levels)
        }
    }

    fn space(&self) -> &'static str {
        if self.config.compact { "" } else { " " }
    }

    pub fn optional_pad(&self, s: &str) -> String {
        if self.config.compact {
            s.to_string()
        } else {
            format!("{}{}{}", self.space(), s, self.space())
        }
    }

    /// Template formatter unchanged (produces a single string which may contain '\n')
    pub fn write_template(&mut self, template: &str, args: &[&str]) {
        let mut result = String::new();
        let mut arg_index = 0;
        let mut in_string = false;
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    result.push(c);
                    in_string = !in_string;
                }
                '%' if !in_string => {
                    if let Some(&next) = chars.peek() {
                        if next == 's' {
                            result.push_str(self.space());
                            chars.next();
                            continue;
                        }
                    }
                    result.push(c);
                }
                '{' if !in_string => {
                    if let Some(&next) = chars.peek() {
                        if next == '}' && arg_index < args.len() {
                            result.push_str(args[arg_index]);
                            arg_index += 1;
                            chars.next();
                            continue;
                        }
                    }
                    result.push(c);
                }
                _ => result.push(c),
            }
        }

        // Use the token-aware writer for the result (handles newlines inside `result`)
        self.write_line(&result);
    }

    /// TOP-LEVEL writer: handles strings that may contain '\n' by splitting and delegating
    /// to `write_wrapped` for each physical line.
    pub fn write_line(&mut self, text: &str) {
        // Split on existing newlines — we respect embedded multi-line subexpressions.
        let mut first = true;
        for segment in text.split('\n') {
            if !first {
                // preserve the explicit newline in the input by writing a blank line with indent.
                // (this keeps block formatting of nested sub-expressions correct)
                // But if segment is empty we should still write an empty indented line.
                self.output.push('\n');
            }
            first = false;
            self.write_wrapped(segment);
        }
    }

    /// Tokenize an input segment (no '\n' expected here).
    fn tokenize(&self, s: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::with_capacity(16);
        let mut it = s.char_indices().peekable();
        while let Some((i, ch)) = it.next() {
            // whitespace token
            if ch.is_whitespace() {
                // collect contiguous whitespace (spaces or tabs)
                let mut end = i + ch.len_utf8();
                while let Some((j, nc)) = it.peek() {
                    if nc.is_whitespace() && *nc != '\n' {
                        let (_, cch) = it.next().unwrap();
                        end += cch.len_utf8();
                    } else {
                        break;
                    }
                }
                let token_text = &s[i..end];
                tokens.push(Token {
                    text: token_text.to_string(),
                    break_after: true, // breaking after whitespace is allowed
                    atomic: false,
                    is_whitespace: true,
                });
                continue;
            }

            // string literal token (supports escaped quotes)
            if ch == '"' {
                let start = i;
                let mut j = i + 1;
                let mut escaped = false;
                while let Some((idx, c2)) = it.next() {
                    j = idx + c2.len_utf8();
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if c2 == '\\' {
                        escaped = true;
                        continue;
                    }
                    if c2 == '"' {
                        break;
                    }
                }
                // if no terminating quote found, take until end
                let token_text = &s[start..j];
                tokens.push(Token {
                    text: token_text.to_string(),
                    break_after: false, // never break inside string or immediately after the quote? we can allow break after if config says (we keep false)
                    atomic: true,
                    is_whitespace: false,
                });
                continue;
            }

            // number (simple heuristics: digit sequence, optional dot and exponent)
            if ch.is_ascii_digit() {
                let start = i;
                let mut end = i + ch.len_utf8();
                while let Some((_, nc)) = it.peek() {
                    if nc.is_ascii_digit()
                        || *nc == '.'
                        || *nc == 'e'
                        || *nc == 'E'
                        || *nc == '+'
                        || *nc == '-'
                    {
                        // for exponent we allow +/-, this is permissive but keeps digits together
                        let (idx, cch) = it.next().unwrap();
                        end = idx + cch.len_utf8();
                    } else {
                        break;
                    }
                }
                let token_text = &s[start..end];
                tokens.push(Token {
                    text: token_text.to_string(),
                    break_after: false, // no break inside numbers; but we could allow break after if next token is punctuation we consider breakable
                    atomic: true,
                    is_whitespace: false,
                });
                continue;
            }

            // identifier (letters, _, digits)
            if ch.is_alphabetic() || ch == '_' {
                let start = i;
                let mut end = i + ch.len_utf8();
                while let Some((_, nc)) = it.peek() {
                    if nc.is_alphanumeric() || *nc == '_' {
                        let (idx, cch) = it.next().unwrap();
                        end = idx + cch.len_utf8();
                    } else {
                        break;
                    }
                }
                let token_text = &s[start..end];
                tokens.push(Token {
                    text: token_text.to_string(),
                    break_after: false,
                    atomic: true,
                    is_whitespace: false,
                });
                continue;
            }

            // operators and punctuation: handle multi-char operators like "==", "->", "::", "&&", "||", "+=", etc.
            {
                let mut two: Option<String> = None;
                if let Some((_, next_ch)) = it.peek() {
                    let candidate = format!("{}{}", ch, next_ch);
                    // set of multi-char ops we want to recognize
                    let multi = [
                        "==", "!=", "<=", ">=", "&&", "||", "::", "->", "=>",
                        "+=", "-=", "*=", "/=", "%=", "<<", ">>",
                    ];
                    if multi.contains(&candidate.as_str()) {
                        two = Some(candidate);
                    }
                }
                if let Some(tok) = two {
                    // consume the second char
                    let _ = it.next();
                    let breakable = self.token_breakable_after_token(&tok);
                    tokens.push(Token {
                        text: tok,
                        break_after: breakable,
                        atomic: false,
                        is_whitespace: false,
                    });
                    continue;
                } else {
                    let single = ch.to_string();
                    let breakable = self.token_breakable_after_char(ch);
                    tokens.push(Token {
                        text: single,
                        break_after: breakable,
                        atomic: false,
                        is_whitespace: false,
                    });
                    continue;
                }
            }
        }

        tokens
    }

    /// decide whether a single-character token is allowed to be followed by a break
    fn token_breakable_after_char(&self, ch: char) -> bool {
        if self.config.allowed_break_after.contains(&ch) {
            true
        } else if self.config.break_after_operators && is_operator_char(ch) {
            true
        } else {
            false
        }
    }

    /// decide whether a multi-char operator token is allowed to be followed by a break
    fn token_breakable_after_token(&self, token: &str) -> bool {
        // allow break after comma/semicolon/close paren/bracket/brace explicitly
        if token.len() == 1 {
            return self
                .token_breakable_after_char(token.chars().next().unwrap());
        }
        // treat most operators as breakable if break_after_operators is true
        if self.config.break_after_operators {
            return true;
        }
        false
    }

    /// Core wrapping algorithm.
    /// - tokenizes the input
    /// - tries to append tokens until `max_line_length`
    /// - when limit exceeded, it finds the last token already appended that has `break_after == true`
    ///   and breaks there. If none, it forces a break before the current token (keeping the token atomic).
    fn write_wrapped(&mut self, segment: &str) {
        // empty -> just write indent and newline
        if segment.is_empty() {
            self.output.push_str(&self.indent_str());
            self.output.push('\n');
            return;
        }

        let tokens = self.tokenize(segment);

        let mut line_buffer = String::new();
        let mut line_len = 0usize;
        // We keep appended tokens to allow searching backwards for breakable token
        let mut appended: Vec<Token> = Vec::with_capacity(tokens.len());

        let indent = self.indent_str();
        let cont_indent = self.continuation_indent_str();

        let max_len = self.config.max_line_length;

        let mut flush_line = |this: &mut Formatter<'a>,
                              buf: &mut String,
                              len: usize,
                              use_cont: bool| {
            if !buf.trim().is_empty() {
                // choose indent for the line: initial indent or continuation indent
                let prefix = if use_cont { &cont_indent } else { &indent };
                this.output.push_str(prefix);
                // trim trailing whitespace to avoid accidental trailing spaces
                let out = buf.trim_end();
                this.output.push_str(out);
                this.output.push('\n');
            } else {
                // even if buffer is whitespace only, write the indent
                let prefix = if use_cont { &cont_indent } else { &indent };
                this.output.push_str(prefix);
                this.output.push('\n');
            }
            buf.clear();
            // return
        };

        // whether the current line is the first physical line (first uses indent, subsequent use continuation)
        let mut is_first_line = true;

        for token in tokens.into_iter() {
            // If the token itself contains no length (shouldn't happen) just skip
            if token.len() == 0 {
                appended.push(token.clone());
                continue;
            }

            // Fast path: if appending token doesn't exceed limit, append
            if line_len + token.len() <= max_len {
                line_buffer.push_str(&token.text);
                line_len += token.len();
                appended.push(token);
                continue;
            }

            // We would exceed max_len by appending this token.
            // Find last appended token index that is breakable.
            let mut break_index: Option<usize> = None;
            for idx in (0..appended.len()).rev() {
                if appended[idx].break_after {
                    break_index = Some(idx);
                    break;
                }
            }

            if let Some(bi) = break_index {
                // build the part to output up to and including appended[bi]
                let mut out_part = String::new();
                for t in appended.iter().take(bi + 1) {
                    out_part.push_str(&t.text);
                }

                // trim trailing whitespace
                let out_part_trimmed = out_part.trim_end().to_string();
                // write the line
                let prefix = if is_first_line {
                    indent.clone()
                } else {
                    cont_indent.clone()
                };
                self.output.push_str(&prefix);
                self.output.push_str(&out_part_trimmed);
                self.output.push('\n');
                is_first_line = false;

                // remaining tokens: those after bi plus current token
                let mut remaining = Vec::new();
                for t in appended.iter().skip(bi + 1) {
                    remaining.push(t.clone());
                }
                remaining.push(token.clone());

                // reset appended and line_buffer to concatenation of remaining tokens
                appended = remaining;
                line_buffer.clear();
                line_len = 0;
                for t in appended.iter() {
                    line_buffer.push_str(&t.text);
                    line_len += t.len();
                }
                // if the current line (remaining) is already too long (single atomic token longer than limit),
                // we still keep the atomic token intact and will emit it on its own line.
                if line_len > max_len {
                    // force flush this long line as-is (we don't split atomic tokens)
                    let prefix = cont_indent.clone();
                    self.output.push_str(&prefix);
                    let out = line_buffer.trim_end();
                    self.output.push_str(out);
                    self.output.push('\n');
                    appended.clear();
                    line_buffer.clear();
                    line_len = 0;
                }
            } else {
                // No safe break point in the current buffer -> force break before current token.
                // Flush current buffer as a line (even if it may be longer than max), then start new line with token.
                let prefix = if is_first_line {
                    indent.clone()
                } else {
                    cont_indent.clone()
                };
                self.output.push_str(&prefix);
                let out = line_buffer.trim_end();
                self.output.push_str(out);
                self.output.push('\n');
                is_first_line = false;

                // clear buffer and set it to current token
                appended.clear();
                line_buffer.clear();
                line_buffer.push_str(&token.text);
                line_len = token.len();
                appended.push(token);

                // If the single token itself is bigger than max, we emit it on its own line (don't split)
                if line_len > max_len {
                    // emit it immediately and clear
                    let prefix = cont_indent.clone();
                    self.output.push_str(&prefix);
                    self.output.push_str(line_buffer.trim_end());
                    self.output.push('\n');
                    appended.clear();
                    line_buffer.clear();
                    line_len = 0;
                }
            }
        }

        // flush remainder
        if !line_buffer.is_empty() {
            let prefix = if is_first_line {
                indent.clone()
            } else {
                cont_indent.clone()
            };
            self.output.push_str(&prefix);
            self.output.push_str(line_buffer.trim_end());
            self.output.push('\n');
        }
    }

    // format_expr_to_string: keep existing behavior: build a sub-formatter and return the result
    fn format_expr_to_string(&self, expr: &DatexExpression) -> String {
        let mut sub_formatter = Formatter::new(self.config);
        sub_formatter.indent_level = self.indent_level;
        sub_formatter.format_ast(expr);
        sub_formatter.output.trim().to_string()
    }

    pub fn format_ast(&mut self, ast: &DatexExpression) {
        match &ast.data {
            DatexExpressionData::Integer(i) => self.write_line(&i.to_string()),
            DatexExpressionData::TypedInteger(ti) => {
                let s = match self.config.variant_suffixes {
                    true => ti.to_string_with_suffix(),
                    false => ti.to_string(),
                };
                self.write_line(&s)
            }
            DatexExpressionData::Decimal(d) => self.write_line(&d.to_string()),
            DatexExpressionData::TypedDecimal(td) => {
                let s = match self.config.variant_suffixes {
                    true => td.to_string_with_suffix(),
                    false => td.to_string(),
                };
                self.write_line(&s)
            }
            DatexExpressionData::Boolean(b) => self.write_line(&b.to_string()),
            DatexExpressionData::Text(t) => {
                self.write_line(&text_to_source_code(t))
            }
            DatexExpressionData::Identifier(l) => self.write_line(&l),
            DatexExpressionData::Null => self.write_line("null"),
            DatexExpressionData::BinaryOperation(op, left, right, _) => {
                self.write_template(
                    "{}%s{}%s{}",
                    &[
                        &self.format_expr_to_string(&left),
                        &op.to_string(),
                        &self.format_expr_to_string(&right),
                    ],
                );
            }
            DatexExpressionData::Statements(statements) => {
                self.write_line("{");
                self.increase_indent();
                for stmt in &statements.statements {
                    self.format_ast(stmt);
                    self.write_line(";");
                }
                self.decrease_indent();
                self.write_line("}");
            }
            DatexExpressionData::List(elements) => {
                if elements.is_empty() {
                    self.write_line("[]");
                    return;
                }

                // Step 1: format all element strings
                let element_strs: Vec<String> = elements
                    .iter()
                    .map(|e| self.format_expr_to_string(e))
                    .collect();

                // Step 2: build a tentative single-line version
                let sep = if self.config.compact { "," } else { ", " };
                let single_line = format!("[{}]", element_strs.join(sep));

                // Step 3: if the single-line version fits, use it
                if single_line.len() <= self.config.max_line_length {
                    self.write_line(&single_line);
                    return;
                }

                // Step 4: otherwise, pretty-print multiline version
                self.write_line("[");
                self.increase_indent();

                for (i, e) in element_strs.iter().enumerate() {
                    let mut line = format!("{}", e);
                    if i < element_strs.len() - 1 {
                        line.push(',');
                    }
                    self.write_line(&line);
                }

                self.decrease_indent();
                self.write_line("]");
            }
            // DatexExpressionData::FunctionDeclaration {
            //     name,
            //     parameters,
            //     return_type,
            //     body,
            // } => {
            //     let params: Vec<String> = parameters
            //         .iter()
            //         .map(|(p, t)| {
            //             format!(
            //                 "{}{}{}",
            //                 p,
            //                 self.optional_pad(":"),
            //                 type_expression_to_source_code(t)
            //             )
            //         })
            //         .collect();
            //     let ret = if let Some(r) = return_type {
            //         format!(
            //             "{}{}",
            //             self.optional_pad("->"),
            //             type_expression_to_source_code(r)
            //         )
            //     } else {
            //         "".to_string()
            //     };
            //     self.write_template(
            //         "fn {}({}){}{}",
            //         &[
            //             name,
            //             &params.join(","),
            //             &ret,
            //             &self.format_expr_to_string(body),
            //         ],
            //     );
            // }
            // DatexExpressionData::Conditional {
            //     condition,
            //     then_branch,
            //     else_branch,
            // } => {
            //     self.write_template(
            //         "if%s{}",
            //         &[&self.format_expr_to_string(condition)],
            //     );
            //     self.format_ast(then_branch);
            //     if let Some(else_branch) = else_branch {
            //         self.write_line("else");
            //         self.format_ast(else_branch);
            //     }
            // }
            // Add all other variants similarly:
            // ApplyChain, VariableDeclaration, TypeDeclaration, etc.
            _ => self.write_line(&format!("{:?}", ast)),
        }
    }

    fn key_to_source_code(&self, key: &DatexExpression) -> String {
        match &key.data {
            DatexExpressionData::Text(t) => self.key_to_string(t),
            DatexExpressionData::Integer(i) => i.to_string(),
            DatexExpressionData::TypedInteger(ti) => ti.to_string(),
            _ => format!("({})", self.format_expr_to_string(key)),
        }
    }

    fn key_to_string(&self, key: &str) -> String {
        // if text does not just contain a-z, A-Z, 0-9, _, and starts with a-z, A-Z,  _, add quotes
        if !self.config.json_compat && is_alphanumeric_identifier(key) {
            key.to_string()
        } else {
            text_to_source_code(key)
        }
    }

    // fn format_expr_to_string(&self, expr: &DatexExpression) -> String {
    //     let mut sub_formatter = Formatter::new(self.config);
    //     sub_formatter.indent_level = self.indent_level;
    //     sub_formatter.format_ast(expr);
    //     sub_formatter.output.trim().to_string()
    // }
}

fn text_to_source_code(text: &str) -> String {
    // escape quotes and backslashes in text
    let text = text
        .replace('\\', r#"\\"#)
        .replace('"', r#"\""#)
        .replace('\u{0008}', r#"\b"#)
        .replace('\u{000c}', r#"\f"#)
        .replace('\r', r#"\r"#)
        .replace('\t', r#"\t"#)
        .replace('\u{000b}', r#"\v"#)
        .replace('\n', r#"\n"#);

    format!("\"{}\"", text)
}

fn is_alphanumeric_identifier(s: &str) -> bool {
    let mut chars = s.chars();

    // First character must be a-z, A-Z, or _
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }

    // Remaining characters: a-z, A-Z, 0-9, _, or -
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Tracks string and bracket depth for safe line breaks
struct LineBreakState {
    in_string: bool,
    paren_depth: usize,
    bracket_depth: usize,
    brace_depth: usize,
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::{
        ast::{binary_operation::BinaryOperator, parse},
        values::core_values::decimal::Decimal,
    };
    const CONFIG_DEFAULT: FormatConfig = FormatConfig {
        compact: false,
        indent_size: 4,
        max_line_length: 120,
        use_spaces: true,
        variant_suffixes: true,
        json_compat: false,
    };

    const CONFIG_COMPACT: FormatConfig = FormatConfig {
        compact: true,
        ..CONFIG_DEFAULT
    };

    const CONFIG_NO_VARIANT_SUFFIXES: FormatConfig = FormatConfig {
        compact: true,
        variant_suffixes: false,
        ..CONFIG_DEFAULT
    };

    const CONFIG_SHORT_LINE: FormatConfig = FormatConfig {
        compact: true,
        max_line_length: 20,
        indent_size: 2,
        ..CONFIG_DEFAULT
    };

    use super::*;

    fn format_ast(ast: &DatexExpression, config: &FormatConfig) -> String {
        let mut formatter = Formatter::new(config);
        formatter.format_ast(ast);
        formatter.output.trim().to_string()
    }
    fn to_expression(s: &str) -> DatexExpression {
        parse(s).unwrap().ast
    }

    #[test]
    fn list() {
        let list = to_expression("[1, 2, 3, 4, 5]");
        assert_eq!(format_ast(&list, &CONFIG_DEFAULT), "[1, 2, 3, 4, 5]");
        assert_eq!(format_ast(&list, &CONFIG_COMPACT), "[1,2,3,4,5]");

        let list = to_expression("[1, 2, 3, 4, 4200000000000000000000000]");
        assert_eq!(
            format_ast(&list, &CONFIG_SHORT_LINE),
            indoc! {"
				[
				  1,
				  2,
				  3,
				  4,
				  4200000000000000000000000
				]"}
        );
    }

    #[test]
    fn arithmetic() {
        // default formatting
        let mut formatter = Formatter::new(&CONFIG_DEFAULT);
        let ast = parse("1 + 2 * 3").unwrap().ast;
        formatter.format_ast(&ast);
        let output = formatter.output.trim();
        assert_eq!(output, "1 + 2 * 3");

        // compact formatting
        let mut formatter = Formatter::new(&CONFIG_COMPACT);
        let ast = parse("1 + 2 * 3").unwrap().ast;
        formatter.format_ast(&ast);
        let output = formatter.output.trim();
        assert_eq!(output, "1+2*3");
    }

    #[test]
    fn typed_variants() {
        let typed_int_ast =
            DatexExpressionData::TypedInteger(42i8.into()).with_default_span();
        assert_eq!(format_ast(&typed_int_ast, &CONFIG_DEFAULT), "42i8");
        assert_eq!(
            format_ast(&typed_int_ast, &CONFIG_NO_VARIANT_SUFFIXES),
            "42"
        );

        let typed_decimal_ast =
            DatexExpressionData::TypedDecimal(2.71f32.into())
                .with_default_span();
        assert_eq!(format_ast(&typed_decimal_ast, &CONFIG_DEFAULT), "2.71f32");
        assert_eq!(
            format_ast(&typed_decimal_ast, &CONFIG_NO_VARIANT_SUFFIXES),
            "2.71"
        );
    }

    #[test]
    fn primitives() {
        let int_ast = DatexExpressionData::Integer(42.into());
        assert_eq!(
            format_ast(&int_ast.with_default_span(), &CONFIG_DEFAULT),
            "42"
        );

        let typed_int_ast = DatexExpressionData::TypedInteger(42i8.into());
        assert_eq!(
            format_ast(&typed_int_ast.with_default_span(), &CONFIG_DEFAULT),
            "42i8"
        );

        let decimal_ast =
            DatexExpressionData::Decimal(Decimal::from_string("1.23").unwrap());
        assert_eq!(
            format_ast(&decimal_ast.with_default_span(), &CONFIG_DEFAULT),
            "1.23"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::Infinity);
        assert_eq!(
            format_ast(&decimal_ast.with_default_span(), &CONFIG_DEFAULT),
            "infinity"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::NegInfinity);
        assert_eq!(
            format_ast(&decimal_ast.with_default_span(), &CONFIG_DEFAULT),
            "-infinity"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::NaN);
        assert_eq!(
            format_ast(&decimal_ast.with_default_span(), &CONFIG_DEFAULT),
            "nan"
        );

        let typed_decimal_ast =
            DatexExpressionData::TypedDecimal(2.71f32.into());
        assert_eq!(
            format_ast(&typed_decimal_ast.with_default_span(), &CONFIG_DEFAULT),
            "2.71f32"
        );

        let bool_ast = DatexExpressionData::Boolean(true);
        assert_eq!(
            format_ast(&bool_ast.with_default_span(), &CONFIG_DEFAULT),
            "true"
        );

        let text_ast = DatexExpressionData::Text("Hello".to_string());
        assert_eq!(
            format_ast(&text_ast.with_default_span(), &CONFIG_DEFAULT),
            "\"Hello\""
        );

        let null_ast = DatexExpressionData::Null;
        assert_eq!(
            format_ast(&null_ast.with_default_span(), &CONFIG_DEFAULT),
            "null"
        );
    }
}

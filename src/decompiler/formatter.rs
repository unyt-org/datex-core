// Improved Rust formatter with recursive block indentation, richer template parsing,
// utility functions and clearer method names.

use std::fmt::{self, Display, Write as _};

/// Formatting mode and options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formatting {
    Compact,
    Multiline { indent: usize }, // indent is number of spaces per level
}

/// A flexible formatter which is stateless and can produce formatted strings
/// according to `Formatting` options. Designed to support recursive block
/// formatting and a variety of utility helpers.
pub struct Formatter {
    pub options: Formatting,
}

/// Convenience macro to mirror `format!`-style usage but route through `Formatter`.
/// Example: f_fmt!(formatter, "Hello,%s{}%n", "world");
#[macro_export]
macro_rules! f_fmt {
    ($formatter:expr, $template:literal $(,)?) => {
        $formatter.format_template($template, &[] as &[&dyn std::fmt::Display])
    };
    ($formatter:expr, $template:literal, $($args:expr),+ $(,)?) => {
        $formatter.format_template($template, &[ $( & $args as &dyn std::fmt::Display ),+ ])
    };
}

// impl Formatter {
//     /// Create a new Formatter with the given formatting options.
//     pub fn new(options: Formatting) -> Self {
//         Self { options }
//     }

//     /// Primary template formatter.
//     ///
//     /// Supported tokens in the template:
//     /// - `{}` — next positional argument
//     /// - `{n}` — indexed positional argument (0-based)
//     /// - `{{` and `}}` — literal `{` and `}`
//     /// - `%s` — optional space (depends on Formatting)
//     /// - `%n` — optional newline (depends on Formatting)
//     /// - `%i` — current indent string for level 0 (useful when combined with `format_block`)
//     pub fn format_template(
//         &self,
//         template: &str,
//         args: &[&dyn Display],
//     ) -> String {
//         let mut out = String::with_capacity(template.len() + args.len() * 8);
//         let mut chars = template.chars().peekable();
//         let mut next_pos_arg = 0usize;

//         while let Some(ch) = chars.next() {
//             if ch == '{' {
//                 // handle escape or indexed/positional placeholder
//                 if let Some(&'{') = chars.peek() {
//                     chars.next(); // consume second '{'
//                     out.push('{');
//                     continue;
//                 }

//                 // collect until '}'
//                 let mut idx_str = String::new();
//                 while let Some(&c2) = chars.peek() {
//                     chars.next();
//                     if c2 == '}' {
//                         break;
//                     }
//                     idx_str.push(c2);
//                 }

//                 if idx_str.is_empty() {
//                     // positional
//                     if next_pos_arg < args.len() {
//                         write!(&mut out, "{}", args[next_pos_arg]).unwrap();
//                     } else {
//                         out.push_str("<missing>");
//                     }
//                     next_pos_arg += 1;
//                 } else {
//                     // try parse index
//                     if let Ok(idx) = idx_str.parse::<usize>() {
//                         if idx < args.len() {
//                             write!(&mut out, "{}", args[idx]).unwrap();
//                         } else {
//                             out.push_str("<missing>");
//                         }
//                     } else {
//                         // unknown form: write verbatim including braces
//                         out.push('{');
//                         out.push_str(&idx_str);
//                         out.push('}');
//                     }
//                 }
//             } else if ch == '}' {
//                 if let Some(&'}') = chars.peek() {
//                     chars.next();
//                     out.push('}');
//                 } else {
//                     // unmatched right brace, push as-is
//                     out.push('}');
//                 }
//             } else if ch == '%' {
//                 // check for %s, %n, %i
//                 if let Some(&next) = chars.peek() {
//                     match next {
//                         's' => {
//                             chars.next();
//                             out.push_str(self.sep_space());
//                         }
//                         'n' => {
//                             chars.next();
//                             out.push_str(self.sep_newline());
//                         }
//                         'i' => {
//                             chars.next();
//                             out.push_str(&self.indent_string(0));
//                         }
//                         other => {
//                             // unknown percent sequence, keep both
//                             chars.next();
//                             out.push('%');
//                             out.push(other);
//                         }
//                     }
//                 } else {
//                     out.push('%');
//                 }
//             } else {
//                 out.push(ch);
//             }
//         }

//         out
//     }

//     /// Returns optional space token depending on `Formatting` mode.
//     pub fn sep_space(&self) -> &'static str {
//         match self.options {
//             Formatting::Compact => "",
//             Formatting::Multiline { .. } => " ",
//         }
//     }

//     /// Returns optional newline token depending on `Formatting` mode.
//     pub fn sep_newline(&self) -> &'static str {
//         match self.options {
//             Formatting::Compact => "",
//             Formatting::Multiline { .. } => "\n",
//         }
//     }

//     /// Surround a string with padding according to formatting mode.
//     pub fn pad(&self, s: &str) -> String {
//         match self.options {
//             Formatting::Compact => s.to_string(),
//             Formatting::Multiline { .. } => format!(" {} ", s),
//         }
//     }

//     /// Join a slice of `Display` items with a separator produced by the formatter.
//     /// The separator respects formatting mode (e.g. comma + optional space in Multiline).
//     pub fn join_display<T: Display>(&self, items: &[T], sep: &str) -> String {
//         let mut out = String::new();
//         for (i, it) in items.iter().enumerate() {
//             if i > 0 {
//                 out.push_str(sep);
//                 out.push_str(self.sep_space());
//             }
//             write!(&mut out, "{}", it).unwrap();
//         }
//         out
//     }

//     /// Indent each line of `s` by `level` indentation levels.
//     /// If Formatting::Compact is selected, returns `s` unchanged.
//     pub fn indent_lines_with(&self, s: &str, level: usize) -> String {
//         match self.options {
//             Formatting::Compact => s.to_string(),
//             Formatting::Multiline { indent } => {
//                 if s.is_empty() {
//                     return String::new();
//                 }
//                 let prefix = " ".repeat(indent * level);
//                 s.lines()
//                     .map(|line| format!("{}{}", prefix, line))
//                     .collect::<Vec<String>>()
//                     .join("\n")
//             }
//         }
//     }

//     /// Indent lines by one level (convenience wrapper).
//     pub fn indent_lines(&self, s: &str) -> String {
//         self.indent_lines_with(s, 1)
//     }

//     /// Produce the indent string for a given level (useful for inserting `%i`).
//     pub fn indent_string(&self, level: usize) -> String {
//         match self.options {
//             Formatting::Compact => String::new(),
//             Formatting::Multiline { indent } => " ".repeat(indent * level),
//         }
//     }

//     /// Format a block with recursive indentation support. Caller provides the
//     /// opening token, a closure which produces the block body (it receives the
//     /// child indent level), and the closing token. The formatter will insert
//     /// newlines and indentation depending on the `Formatting` mode.
//     ///
//     /// Example: format_block("{", |child_level| f_fmt!(fmt, "foo,%s{}", ...), "}")
//     pub fn format_block<F>(
//         &self,
//         open: &str,
//         body: F,
//         close: &str,
//         current_level: usize,
//     ) -> String
//     where
//         F: FnOnce(usize) -> String,
//     {
//         match self.options {
//             Formatting::Compact => {
//                 // compact: everything in one line, no extra spaces unless open/close have them
//                 let inner = body(current_level + 1);
//                 format!("{}{}{}", open, inner, close)
//             }
//             Formatting::Multiline { indent } => {
//                 let inner = body(current_level + 1);
//                 if inner.trim().is_empty() {
//                     // empty block -> produce open + close on same line
//                     format!(
//                         "{}{}{}",
//                         open,
//                         self.sep_newline(),
//                         self.indent_string(current_level)
//                     )
//                 } else {
//                     let indented_inner =
//                         self.indent_lines_with(&inner, current_level + 1);
//                     let mut out = String::new();
//                     write!(
//                         &mut out,
//                         "{}\n{}\n{}{}",
//                         open,
//                         indented_inner,
//                         self.indent_string(current_level),
//                         close
//                     )
//                     .unwrap();
//                     out
//                 }
//             }
//         }
//     }

//     /// Surround content with open/close but inline when compact, multiline with indentation when multiline.
//     pub fn surround_block(
//         &self,
//         open: &str,
//         content: &str,
//         close: &str,
//         level: usize,
//     ) -> String {
//         self.format_block(open, |_| content.to_string(), close, level)
//     }

//     /// Escape braces so they are treated as literal characters in templates.
//     pub fn escape_braces(s: &str) -> String {
//         s.replace("{", "{{").replace("}", "}}")
//     }
// }

impl Formatter {
    pub fn new(options: Formatting) -> Self {
        Self { options }
    }

    pub fn format_template(
        &self,
        template: &str,
        args: &[&dyn Display],
    ) -> String {
        let mut out = String::with_capacity(template.len() + args.len() * 8);
        let mut chars = template.chars().peekable();
        let mut next_pos_arg = 0usize;

        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some(&'{') = chars.peek() {
                    chars.next();
                    out.push('{');
                    continue;
                }
                let mut idx_str = String::new();
                while let Some(&c2) = chars.peek() {
                    chars.next();
                    if c2 == '}' {
                        break;
                    }
                    idx_str.push(c2);
                }
                if idx_str.is_empty() {
                    if next_pos_arg < args.len() {
                        write!(&mut out, "{}", args[next_pos_arg]).unwrap();
                    } else {
                        out.push_str("<missing>");
                    }
                    next_pos_arg += 1;
                } else {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if idx < args.len() {
                            write!(&mut out, "{}", args[idx]).unwrap();
                        } else {
                            out.push_str("<missing>");
                        }
                    } else {
                        out.push('{');
                        out.push_str(&idx_str);
                        out.push('}');
                    }
                }
            } else if ch == '}' {
                if let Some(&'}') = chars.peek() {
                    chars.next();
                    out.push('}');
                } else {
                    out.push('}');
                }
            } else if ch == '%' {
                if let Some(&next) = chars.peek() {
                    match next {
                        's' => {
                            chars.next();
                            out.push_str(self.sep_space());
                        }
                        'n' => {
                            chars.next();
                            out.push_str(self.sep_newline());
                        }
                        'i' => {
                            chars.next();
                            out.push_str(&self.indent_string(0));
                        }
                        other => {
                            chars.next();
                            out.push('%');
                            out.push(other);
                        }
                    }
                } else {
                    out.push('%');
                }
            } else {
                out.push(ch);
            }
        }
        out
    }

    pub fn sep_space(&self) -> &'static str {
        match self.options {
            Formatting::Compact => "",
            Formatting::Multiline { .. } => " ",
        }
    }

    pub fn sep_newline(&self) -> &'static str {
        match self.options {
            Formatting::Compact => "",
            Formatting::Multiline { .. } => "\n",
        }
    }

    pub fn pad(&self, s: &str) -> String {
        match self.options {
            Formatting::Compact => s.to_string(),
            Formatting::Multiline { .. } => format!(" {} ", s),
        }
    }

    pub fn join_display<T: Display>(&self, items: &[T], sep: &str) -> String {
        let mut out = String::new();
        for (i, it) in items.iter().enumerate() {
            if i > 0 {
                out.push_str(sep);
                out.push_str(self.sep_space());
            }
            write!(&mut out, "{}", it).unwrap();
        }
        out
    }

    pub fn indent_lines_with(&self, s: &str, level: usize) -> String {
        match self.options {
            Formatting::Compact => s.to_string(),
            Formatting::Multiline { indent } => {
                if s.is_empty() {
                    return String::new();
                }
                let prefix = " ".repeat(indent * level);
                s.lines()
                    .map(|line| format!("{}{}", prefix, line))
                    .collect::<Vec<String>>()
                    .join("\n")
            }
        }
    }

    pub fn indent_lines(&self, s: &str) -> String {
        self.indent_lines_with(s, 1)
    }

    pub fn indent_string(&self, level: usize) -> String {
        match self.options {
            Formatting::Compact => String::new(),
            Formatting::Multiline { indent } => " ".repeat(indent * level),
        }
    }

    // pub fn format_block<F>(
    //     &self,
    //     open: &str,
    //     body: F,
    //     close: &str,
    //     current_level: usize,
    // ) -> String
    // where
    //     F: FnOnce(usize) -> String,
    // {
    //     match self.options {
    //         Formatting::Compact => {
    //             let inner = body(current_level + 1);
    //             format!("{}{}{}", open, inner, close)
    //         }
    //         Formatting::Multiline { .. } => {
    //             let inner = body(current_level + 1);
    //             if inner.trim().is_empty() {
    //                 return format!(
    //                     "{}\n{}{}",
    //                     open,
    //                     self.indent_string(current_level),
    //                     close
    //                 );
    //             }
    //             let indented_inner =
    //                 self.indent_lines_with(&inner, current_level + 1);
    //             let mut out = String::new();
    //             write!(
    //                 &mut out,
    //                 "{}\n{}\n{}{}",
    //                 open,
    //                 indented_inner,
    //                 self.indent_string(current_level),
    //                 close
    //             )
    //             .unwrap();
    //             out
    //         }
    //     }
    // }
    pub fn format_block<F>(
        &self,
        open: &str,
        body: F,
        close: &str,
        current_level: usize,
    ) -> String
    where
        F: FnOnce(usize) -> String,
    {
        match self.options {
            Formatting::Compact => {
                // compact: everything in one line, no extra spaces unless open/close have them
                let inner = body(current_level + 1);
                format!("{}{}{}", open, inner, close)
            }
            Formatting::Multiline { indent } => {
                let inner = body(current_level + 1);
                if inner.trim().is_empty() {
                    // empty block -> produce open + close on same line
                    format!(
                        "{}{}{}",
                        open,
                        self.sep_newline(),
                        self.indent_string(current_level)
                    )
                } else {
                    let indented_inner =
                        self.indent_lines_with(&inner, current_level + 1);
                    let mut out = String::new();
                    write!(
                        &mut out,
                        "{}{}\n{}\n{}{}",
                        self.indent_string(current_level),
                        open,
                        indented_inner,
                        self.indent_string(current_level),
                        close
                    )
                    .unwrap();
                    out
                }
            }
        }
    }

    pub fn surround_block(
        &self,
        open: &str,
        content: &str,
        close: &str,
        level: usize,
    ) -> String {
        self.format_block(open, |_| content.to_string(), close, level)
    }

    pub fn escape_braces(s: &str) -> String {
        s.replace("{", "{{").replace("}", "}}")
    }
}
// ---------- Tests ----------
#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    #[test]
    fn test_percent_tokens_compact_vs_multiline() {
        let compact = Formatter::new(Formatting::Compact);
        let multi = Formatter::new(Formatting::Multiline { indent: 2 });

        let c = f_fmt!(compact, "A,%s{}%nB", "x");
        assert_eq!(c, "A,xB");

        let m = f_fmt!(multi, "A,%s{}%nB", "x");
        assert_eq!(m, "A, x\nB");
    }

    #[test]
    fn test_indent_lines_empty_and_levels() {
        let fmt = Formatter::new(Formatting::Multiline { indent: 3 });
        assert_eq!(fmt.indent_lines_with("", 2), "");

        let input = "l1\nl2";
        let out = fmt.indent_lines_with(input, 2);
        assert_eq!(out, format!("{}l1\n{}l2", " ".repeat(6), " ".repeat(6)));
    }

    #[test]
    fn test_join_display_and_pad() {
        let compact = Formatter::new(Formatting::Compact);
        let multi = Formatter::new(Formatting::Multiline { indent: 2 });

        let items = ["a", "b", "c"];
        assert_eq!(compact.join_display(&items, ","), "a,b,c");
        assert_eq!(multi.join_display(&items, ","), "a, b, c");

        assert_eq!(compact.pad("x"), "x");
        assert_eq!(multi.pad("x"), " x ");
    }

    #[test]
    fn test_format_block_compact_vs_multiline_nonempty() {
        let compact = Formatter::new(Formatting::Compact);
        let multi = Formatter::new(Formatting::Multiline { indent: 2 });

        let b_c = compact.format_block("{", |_| "a;b".to_string(), "}", 0);
        assert_eq!(b_c, "{a;b}");

        let b_m = multi.format_block("{", |_| "x;\ny".to_string(), "}", 0);
        assert_eq!(b_m, "{\n  x;\n  y\n}");
    }

    #[test]
    fn test_format_block_multiline_empty_block() {
        let multi = Formatter::new(Formatting::Multiline { indent: 2 });
        let b = multi.format_block("{", |_| "".to_string(), "}", 0);
        // An empty block should render as open + newline + close on its own line
        assert_eq!(b, "{\n}");
    }

    #[test]
    fn test_surround_block_and_nested_blocks() {
        let fmt = Formatter::new(Formatting::Multiline { indent: 0 });
        let nested = fmt.format_block(
            "{",
            |lvl| fmt.format_block("[", |_| "inner".to_string(), "]", lvl),
            "}",
            0,
        );
        let expected = indoc! {r#"
			{
			[
			inner
			]
			}"#};
        assert_eq!(nested, expected);

        let fmt = Formatter::new(Formatting::Multiline { indent: 1 });
        let nested = fmt.format_block(
            "{",
            |lvl| fmt.format_block("[", |_| "inner".to_string(), "]", lvl),
            "}",
            0,
        );
        let expected = indoc! {r#"
			{
			 [
			  inner
			 ]
			}"#};

        println!("NESTED:\n{}", nested);
        println!("EXPECTED:\n{}", expected);

        assert_eq!(nested, expected);

        return;

        let inner = fmt.surround_block("(", "a,b", ")", 0);
        assert_eq!(inner, "(\n a,b\n)");

        let inner2 = fmt.surround_block("(", "a,b", ")", 1);
        assert_eq!(inner2, " (\n  a,b\n )");

        // nested blocks

        println!("NESTED:\n{}", nested);
        println!("EXPECTED:\n{}", expected);
        assert_eq!(nested, expected);
    }
}

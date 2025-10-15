#[macro_export]
macro_rules! f_fmt {
    ($formatter:expr, $template:literal $(,)?) => {
        $formatter.fmt($template, &[] as &[&dyn std::fmt::Display])
    };
    ($formatter:expr, $template:literal, $($args:expr),+ $(,)?) => {
        $formatter.fmt($template, &[ $( & $args as &dyn std::fmt::Display ),+ ])
    };
}

pub struct Formatter {
    options: Formatting,
}
use std::fmt::Write;

use crate::decompiler::Formatting;

impl Formatter {
    pub fn new(options: Formatting) -> Self {
        Formatter { options }
    }

    pub fn fmt(
        &self,
        template: &str,
        args: &[&dyn std::fmt::Display],
    ) -> String {
        let mut result = String::new();

        let mut formatted_template =
            template.replace("%s", self.optional_space());
        formatted_template =
            formatted_template.replace("%n", self.optional_newline());

        let mut parts = formatted_template.split("{}");
        if let Some(first) = parts.next() {
            result.push_str(first);
        }

        for (part, arg) in parts.zip(args.iter()) {
            write!(result, "{}", arg).unwrap();
            result.push_str(part);
        }

        result
    }

    pub fn optional_space(&self) -> &str {
        match self.options {
            Formatting::Compact => "",
            Formatting::Multiline { .. } => " ",
        }
    }

    pub fn optional_newline(&self) -> &str {
        match self.options {
            Formatting::Compact => "",
            Formatting::Multiline { .. } => "\n",
        }
    }

    pub fn optional_pad(&self, s: &str) -> String {
        match self.options {
            Formatting::Compact => s.to_string(),
            Formatting::Multiline { .. } => {
                format!(" {} ", s)
            }
        }
    }

    /// Indents each line of the given string by the specified number of spaces if multiline formatting is used
    pub fn indent_lines(&self, s: &str) -> String {
        match self.options {
            Formatting::Compact => s.to_string(),
            Formatting::Multiline { indent } => s
                .lines()
                .map(|line| format!("{}{}", " ".repeat(indent), line))
                .collect::<Vec<String>>()
                .join("\n"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fmt_compact() {
        let formatter = Formatter::new(Formatting::Compact);
        let result = f_fmt!(formatter, "Hello,%s{}!%n", "world");
        assert_eq!(result, "Hello,world!");
    }

	#[test]
	fn test_fmt_multiline() {
		let formatter = Formatter::new(Formatting::Multiline { indent: 4 });
		let result = f_fmt!(formatter, "Hello,%s{}!%n", "world");
		assert_eq!(result, "Hello, world!\n");
	}

	#[test]
	fn test_indent_lines_compact() {
		let formatter = Formatter::new(Formatting::Compact);
		let input = "line1\nline2\nline3";
		let result = formatter.indent_lines(input);
		assert_eq!(result, input);
	}

	#[test]
	fn test_indent_lines_multiline() {
		let formatter = Formatter::new(Formatting::Multiline { indent: 2 });
		let input = "line1\nline2\nline3";
		let result = formatter.indent_lines(input);
		let expected = "  line1\n  line2\n  line3";
		assert_eq!(result, expected);
	}
}

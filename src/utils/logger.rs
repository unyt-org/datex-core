use std::{sync::Mutex, cell::Cell};

use crate::datex_values::Value;

use super::color::{Color, AnsiCodes};

pub struct Logger<'a> {
	name: String,
	is_production: bool,
	formatting: LOG_FORMATTING,
	context: &'a LoggerContext
}

#[derive(Clone, Copy)]
pub enum LOG_LEVEL {
    VERBOSE,
    DEFAULT,
    WARNING,
    ERROR,
}

#[derive(PartialEq)]
pub enum LOG_FORMATTING {
    PLAINTEXT,
    COLOR_4_BIT,
    COLOR_RGB
}

pub struct LoggerContext {
	pub log_redirect: Option<fn(&str)->()>
}

static development_log_level:u8 = LOG_LEVEL::VERBOSE as u8;
static production_log_level:u8 = LOG_LEVEL::VERBOSE  as u8;


impl Logger<'_> {


	pub fn new<'a>(context: &'a LoggerContext, name:&'a str, is_production:bool, formatting:LOG_FORMATTING) -> Logger<'a> {
		return Logger {name:(*name).to_string(), is_production, formatting, context}
	}
	pub fn new_for_production<'a>(context: &'a LoggerContext, name:&'a str) -> Logger<'a> {
		return Logger {name:(*name).to_string(), is_production:true, formatting:LOG_FORMATTING::COLOR_RGB, context}
	}
	pub fn new_for_development<'a>(context: &'a LoggerContext, name:&'a str) -> Logger<'a> {
		return Logger {name:(*name).to_string(), is_production:false, formatting:LOG_FORMATTING::COLOR_RGB, context}
	}



	fn log(&self, text:&str, data: &Vec<Box<dyn Value>>, color:Color, log_level:LOG_LEVEL, only_log_own_stream:bool, add_tag:bool) {
		if !self.log_level_allowed(log_level.clone()) {return}

		let formatted = self.generate_log_string(text, data, color, add_tag);
		self.log_raw(&formatted, log_level, only_log_own_stream);
	}

	fn generate_log_string(&self, text:&str, data: &Vec<Box<dyn Value>>, color:Color, add_tag:bool) -> String {
		let message = text;
		let end = if self.formatting == LOG_FORMATTING::PLAINTEXT {""} else {AnsiCodes::RESET};

		if add_tag {format!("{}{}{}", self.get_tag(color), message, end)}
		else {format!("{}{}", message, end)}
	}

	fn log_raw(&self, text:&str, log_level:LOG_LEVEL, only_log_own_stream:bool) {

        if !self.log_level_allowed(log_level) {return}


		let handler = self.context.log_redirect;

		// log handler
		if handler.is_some() {(handler.as_ref().unwrap())(text)}
		// default std out
		else {println!("{}", text)}
	}

	// check if the current production/development log level includes a log level
	fn log_level_allowed(&self, log_level:LOG_LEVEL) -> bool {
		let log_level_u8 = log_level as u8;
		
		if self.is_production && (log_level_u8 < production_log_level) {false} // don't log for production
        else if !self.is_production && (log_level_u8 < development_log_level) {false} // don't log for development
		else {true}
	}

	fn get_tag(&self, color:Color) -> String {
		let color_esc = self.get_formatting_color(color);
		let mut tag = "".to_string();

        // handle tag:
        let esc_tag = self.formatting != LOG_FORMATTING::PLAINTEXT;

        // start tag
        if esc_tag {
			let end = if self.formatting == LOG_FORMATTING::COLOR_RGB {AnsiCodes::BOLD.to_string()} else {"".to_string()};
            tag += &format!("{}{}{}{}", AnsiCodes::INVERSE, AnsiCodes::UNDERLINE, color_esc, end);
        }

		if self.formatting == LOG_FORMATTING::PLAINTEXT {tag += &format!("[{}]", self.name)}
        else {tag +=  &format!(" {} ", self.name)}

        // tag content
        // if (this.origin) {
        //     if (this.formatting == LOG_FORMATTING.PLAINTEXT) tag += `[${this.origin}]`;
        //     else tag +=  " " + this.origin  + " ";
        // }
        // if (this.pointer) {
        //     if (this.formatting == LOG_FORMATTING.PLAINTEXT) tag += `[${this.pointer}]`;
        //     else tag += ESCAPE_SEQUENCES.INVERSE+ESCAPE_SEQUENCES.UNDERLINE + this.getFormattingColor(COLOR.POINTER) + " " + this.pointer + " ";
        // }
        // end tag
        if esc_tag {tag += &format!("{} {}", AnsiCodes::RESET, color_esc)}

        return tag;
	}

	fn get_formatting_color(&self, color:Color) -> String {
        if self.formatting == LOG_FORMATTING::COLOR_4_BIT {return color.as_ansi_4_bit().to_string()}
        else if self.formatting == LOG_FORMATTING::COLOR_RGB {return color.as_ansi_rgb()}
        else if self.formatting == LOG_FORMATTING::PLAINTEXT {return "".to_string()}
        else {return AnsiCodes::COLOR_DEFAULT.to_string()}
    }

	// public log methods

	pub fn success(&self, message:&str) {
		self.log(message, &Vec::new(), Color::GREEN, LOG_LEVEL::DEFAULT, false, true);
	}

	pub fn error(&self, message:&str) {
		self.log(message, &Vec::new(), Color::RED, LOG_LEVEL::ERROR, false, true);
	}

	pub fn warn(&self, message:&str) {
		self.log(message, &Vec::new(), Color::YELLOW, LOG_LEVEL::WARNING, false, true);
	}

	pub fn info(&self, message:&str) {
		self.log(message, &Vec::new(), Color::DEFAULT, LOG_LEVEL::DEFAULT, false, true);
	}

	pub fn debug(&self, message:&str) {
		self.log(message, &Vec::new(), Color::CYAN, LOG_LEVEL::VERBOSE, false, true);
	}

	pub fn plain(&self, message:&str) {
		self.log(message, &Vec::new(), Color::WHITE, LOG_LEVEL::DEFAULT, false, false);
	}
}
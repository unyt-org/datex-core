use std::{
  cell::RefCell,
  rc::Rc,
  sync::{Arc, Mutex},
};

use crate::datex_values::Value;

use super::color::{AnsiCodes, Color};

#[derive(Clone, Debug)]
pub struct Logger {
  name: String,
  is_production: bool,
  formatting: LogFormatting,
  context: Rc<RefCell<LoggerContext>>,
}

#[derive(Clone, Copy)]
pub enum LogLevel {
  VERBOSE,
  DEFAULT,
  WARNING,
  ERROR,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum LogFormatting {
  PlainText,
  Color4Bit,
  ColorRGB,
}

#[derive(Debug)]
pub struct LoggerContext {
  pub log_redirect: Option<fn(&str) -> ()>,
}

static DEVELOPMENT_LOG_LEVEL: u8 = LogLevel::VERBOSE as u8;
static PRODUCTION_LOG_LEVEL: u8 = LogLevel::VERBOSE as u8;

impl Logger {
  pub fn new(
    context: Rc<RefCell<LoggerContext>>,
    name: String,
    is_production: bool,
    formatting: LogFormatting,
  ) -> Logger {
    return Logger {
      name: (*name).to_string(),
      is_production,
      formatting,
      context,
    };
  }
  pub fn new_for_production(
    context: Rc<RefCell<LoggerContext>>,
    name: String,
  ) -> Logger {
    return Logger {
      name: (*name).to_string(),
      is_production: true,
      formatting: LogFormatting::ColorRGB,
      context,
    };
  }
  pub fn new_for_development<'a>(
    context: Rc<RefCell<LoggerContext>>,
    name: String,
  ) -> Logger {
    return Logger {
      name: (*name).to_string(),
      is_production: false,
      formatting: LogFormatting::ColorRGB,
      context,
    };
  }

  fn log(
    &self,
    text: &str,
    data: &Vec<Box<dyn Value>>,
    color: Color,
    log_level: LogLevel,
    only_log_own_stream: bool,
    add_tag: bool,
  ) {
    if !self.log_level_allowed(log_level.clone()) {
      return;
    }

    let formatted = self.generate_log_string(text, data, color, add_tag);
    self.log_raw(&formatted, log_level, only_log_own_stream);
  }

  fn generate_log_string(
    &self,
    text: &str,
    _data: &Vec<Box<dyn Value>>,
    color: Color,
    add_tag: bool,
  ) -> String {
    let message = text;
    let end = if self.formatting == LogFormatting::PlainText {
      ""
    } else {
      AnsiCodes::RESET
    };

    if add_tag {
      format!("{}{}{}", self.get_tag(color), message, end)
    } else {
      format!("{}{}", message, end)
    }
  }

  fn log_raw(
    &self,
    text: &str,
    log_level: LogLevel,
    _only_log_own_stream: bool,
  ) {
    if !self.log_level_allowed(log_level) {
      return;
    }

    let handler = self.context.borrow().log_redirect;

    // log handler
    if handler.is_some() {
      (handler.as_ref().unwrap())(text)
    }
    // default std out
    else {
      println!("{}", text)
    }
  }

  // check if the current production/development log level includes a log level
  fn log_level_allowed(&self, log_level: LogLevel) -> bool {
    let log_level_u8 = log_level as u8;

    if self.is_production && (log_level_u8 < PRODUCTION_LOG_LEVEL) {
      false
    }
    // don't log for production
    else if !self.is_production && (log_level_u8 < DEVELOPMENT_LOG_LEVEL) {
      false
    }
    // don't log for development
    else {
      true
    }
  }

  fn get_tag(&self, color: Color) -> String {
    let color_esc = self.get_formatting_color(color);
    let mut tag = "".to_string();

    // handle tag:
    let esc_tag = self.formatting != LogFormatting::PlainText;

    // start tag
    if esc_tag {
      let end = if self.formatting == LogFormatting::ColorRGB {
        AnsiCodes::BOLD.to_string()
      } else {
        "".to_string()
      };
      tag += &format!(
        "{}{}{}{}{}",
        AnsiCodes::INVERSE,
        AnsiCodes::UNDERLINE,
        self.get_formatting_color_bg(Color::BLACK),
        color_esc,
        end
      );
    }

    if self.formatting == LogFormatting::PlainText {
      tag += &format!("[{}]", self.name)
    } else {
      tag += &format!(" {} ", self.name)
    }

    // tag content
    // if (this.origin) {
    //     if (this.formatting == LogFormatting.PLAINTEXT) tag += `[${this.origin}]`;
    //     else tag +=  " " + this.origin  + " ";
    // }
    // if (this.pointer) {
    //     if (this.formatting == LogFormatting.PLAINTEXT) tag += `[${this.pointer}]`;
    //     else tag += ESCAPE_SEQUENCES.INVERSE+ESCAPE_SEQUENCES.UNDERLINE + this.getFormattingColor(COLOR.POINTER) + " " + this.pointer + " ";
    // }
    // end tag
    if esc_tag {
      tag += &format!("{} {}", AnsiCodes::RESET, color_esc)
    }

    return tag;
  }

  fn get_formatting_color(&self, color: Color) -> String {
    if self.formatting == LogFormatting::Color4Bit {
      return color.as_ansi_4_bit().to_string();
    } else if self.formatting == LogFormatting::ColorRGB {
      return color.as_ansi_rgb();
    } else if self.formatting == LogFormatting::PlainText {
      return "".to_string();
    } else {
      return AnsiCodes::COLOR_DEFAULT.to_string();
    }
  }

  fn get_formatting_color_bg(&self, color: Color) -> String {
    if self.formatting == LogFormatting::Color4Bit {
      return color.as_ansi_4_bit_bg().to_string();
    } else if self.formatting == LogFormatting::ColorRGB {
      return color.as_ansi_rgb_bg();
    } else if self.formatting == LogFormatting::PlainText {
      return "".to_string();
    } else {
      return AnsiCodes::COLOR_DEFAULT.to_string();
    }
  }

  // public log methods

  pub fn success(&self, message: &str) {
    self.log(
      message,
      &Vec::new(),
      Color::GREEN,
      LogLevel::DEFAULT,
      false,
      true,
    );
  }

  pub fn error(&self, message: &str) {
    self.log(
      message,
      &Vec::new(),
      Color::RED,
      LogLevel::ERROR,
      false,
      true,
    );
  }

  pub fn warn(&self, message: &str) {
    self.log(
      message,
      &Vec::new(),
      Color::YELLOW,
      LogLevel::WARNING,
      false,
      true,
    );
  }

  pub fn info(&self, message: &str) {
    self.log(
      message,
      &Vec::new(),
      Color::DEFAULT,
      LogLevel::DEFAULT,
      false,
      true,
    );
  }

  pub fn debug(&self, message: &str) {
    self.log(
      message,
      &Vec::new(),
      Color::CYAN,
      LogLevel::VERBOSE,
      false,
      true,
    );
  }

  pub fn plain(&self, message: &str) {
    self.log(
      message,
      &Vec::new(),
      Color::WHITE,
      LogLevel::DEFAULT,
      false,
      false,
    );
  }
}

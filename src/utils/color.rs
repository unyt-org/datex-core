pub struct AnsiCodes {}
impl AnsiCodes {
  pub const COLOR_DEFAULT: &'static str = "\x1b[39m";

  pub const CLEAR: &'static str = "\x1b[2J"; // clear screen

  pub const RESET: &'static str = "\x1b[0m";
  pub const BOLD: &'static str = "\x1b[1m";
  pub const DEFAULT: &'static str = "\x1b[2m";
  pub const ITALIC: &'static str = "\x1b[3m";
  pub const UNDERLINE: &'static str = "\x1b[4m";
  pub const INVERSE: &'static str = "\x1b[7m";
  pub const HIDDEN: &'static str = "\x1b[8m";

  pub const RESET_UNDERLINE: &'static str = "\x1b[24m";
  pub const RESET_INVERSE: &'static str = "\x1b[27m";

  pub const BLACK: &'static str = "\x1b[30m";
  pub const RED: &'static str = "\x1b[31m";
  pub const GREEN: &'static str = "\x1b[32m";
  pub const YELLOW: &'static str = "\x1b[33m";
  pub const BLUE: &'static str = "\x1b[34m";
  pub const MAGENTA: &'static str = "\x1b[35m";
  pub const CYAN: &'static str = "\x1b[36m";
  pub const WHITE: &'static str = "\x1b[37m";
  pub const GREY: &'static str = "\x1b[90m";

  pub const BG_BLACK: &'static str = "\x1b[40m";
  pub const BG_RED: &'static str = "\x1b[41m";
  pub const BG_GREEN: &'static str = "\x1b[42m";
  pub const BG_YELLOW: &'static str = "\x1b[43m";
  pub const BG_BLUE: &'static str = "\x1b[44m";
  pub const BG_MAGENTA: &'static str = "\x1b[45m";
  pub const BG_CYAN: &'static str = "\x1b[46m";
  pub const BG_WHITE: &'static str = "\x1b[47m";
  pub const BG_GREY: &'static str = "\x1b[100m";
  pub const BG_COLOR_DEFAULT: &'static str = "\x1b[49m";
}

#[derive(PartialEq)]
pub enum Color {
  RED,
  GREEN,
  BLUE,
  YELLOW,
  MAGENTA,
  CYAN,
  BLACK,
  WHITE,
  GREY,

  TEXT,
  NUMBER,
  BUFFER,
  PrimitiveConstant,
  TYPE,
  TIME,

  DEFAULT,
  DefaultLight,
  RESERVED,

  ENDPOINT,
  EndpointPerson,
  EndpointInstitution,

  _UNKNOWN, // imply further color resolution
}

pub fn ansi_rgb(r: u8, g: u8, b: u8) -> String {
  return format!("\x1b[38;2;{r};{g};{b}m");
}

impl Color {
  pub fn as_ansi_rgb_bg(&self) -> String {
    self
      .as_ansi_rgb()
      .as_str()
      .replacen("38", "48", 1)
      .to_string()
  }

  pub fn as_ansi_rgb(&self) -> String {
    match self {
      Color::RED => ansi_rgb(234, 43, 81),
      Color::GREEN => ansi_rgb(30, 218, 109),
      Color::BLUE => ansi_rgb(6, 105, 193),
      Color::YELLOW => ansi_rgb(235, 182, 38),
      Color::MAGENTA => ansi_rgb(196, 112, 222),
      Color::CYAN => ansi_rgb(79, 169, 232),
      Color::BLACK => ansi_rgb(5, 5, 5),
      Color::WHITE => ansi_rgb(250, 250, 250),
      Color::GREY => ansi_rgb(150, 150, 150),

      Color::TEXT => ansi_rgb(183, 129, 227),
      Color::NUMBER => ansi_rgb(253, 139, 25),
      Color::PrimitiveConstant => ansi_rgb(219, 45, 129),
      Color::BUFFER => ansi_rgb(238, 95, 95),
      Color::TYPE => ansi_rgb(50, 153, 220),
      Color::TIME => ansi_rgb(253, 213, 25),

      Color::ENDPOINT => ansi_rgb(24, 219, 164),
      Color::EndpointPerson => ansi_rgb(41, 199, 61),
      Color::EndpointInstitution => ansi_rgb(135, 201, 36),

      Color::RESERVED => ansi_rgb(65, 102, 238),
      Color::DEFAULT => AnsiCodes::COLOR_DEFAULT.to_string(),
      Color::DefaultLight => ansi_rgb(173, 173, 173),

      Color::_UNKNOWN => ansi_rgb(255, 0, 255), // invalid: magenta
    }
  }

  // TODO:
  pub fn as_ansi_4_bit_bg(&self) -> &'static str {
    match self {
      _ => "",
    }
  }

  // TODO:
  pub fn as_ansi_4_bit(&self) -> &'static str {
    match self {
      _ => "",
    }
  }
}

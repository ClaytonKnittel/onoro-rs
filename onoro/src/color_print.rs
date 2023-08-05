use std::{borrow::Borrow, fmt::Display};

/*

#define P_BLACK   "\033[0;30m"
#define P_RED     "\033[0;31m"
#define P_GREEN   "\033[0;32m"
#define P_YELLOW  "\033[0;33m"
#define P_BLUE    "\033[0;34m"
#define P_MAGENTA "\033[0;35m"
#define P_CYAN    "\033[0;36m"
#define P_WHITE   "\033[0;37m"
#define P_DEFAULT "\033[0;39m"

#define P_LGRAY    "\033[0;37m"
#define P_DGRAY    "\033[0;90m"
#define P_LRED     "\033[0;91m"
#define P_LGREEN   "\033[0;92m"
#define P_LYELLOW  "\033[0;93m"
#define P_LBLUE    "\033[0;94m"
#define P_LMAGENTA "\033[0;95m"
#define P_LCYAN    "\033[0;96m"
#define P_LWHITE   "\033[0;97m"
*/

pub enum Color {
  Default,
  Black,
  Red,
  Green,
  Yellow,
  Blue,
  Magenta,
  Cyan,
  White,
}

#[derive(Default)]
pub struct ColorAttrs {
  pub light: bool,
  pub bold: bool,
}

pub struct Colored<T> {
  val: T,
  color: Color,
  attrs: ColorAttrs,
}

impl<T> Colored<T> {
  pub fn new(val: T, color: Color) -> Self {
    Self::with_attrs(val, color, ColorAttrs::default())
  }

  pub fn with_attrs(val: T, color: Color, attrs: ColorAttrs) -> Self {
    Self { val, color, attrs }
  }
}

impl<T> Borrow<T> for Colored<T> {
  fn borrow(&self) -> &T {
    &self.val
  }
}

impl<T> From<T> for Colored<T> {
  fn from(value: T) -> Self {
    Self::new(value, Color::Default)
  }
}

impl<T> Display for Colored<T>
where
  T: Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let (color_code, reset_color_code) = match match self.color {
      Color::Black => Some(30),
      Color::Red => Some(31),
      Color::Green => Some(32),
      Color::Yellow => Some(33),
      Color::Blue => Some(34),
      Color::Magenta => Some(35),
      Color::Cyan => Some(36),
      Color::White => Some(37),
      Color::Default => None,
    }
    .map(|code| if self.attrs.light { code + 60 } else { code })
    {
      Some(color_code) => (format!("\x1b[0;{color_code}m"), "\x1b[0;39m".to_owned()),
      None => ("".to_owned(), "".to_owned()),
    };
    let (bold_code, reset_bold_code) = if self.attrs.bold {
      ("\x1b[1m", "\x1b[21m")
    } else {
      ("", "")
    };

    write!(
      f,
      "{color_code}{bold_code}{}{reset_bold_code}{reset_color_code}",
      self.val,
    )
  }
}

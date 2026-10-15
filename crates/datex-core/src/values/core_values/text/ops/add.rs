use core::ops::Add;

use crate::values::core_values::text::Text;

impl Add for &Text {
    type Output = Text;

    fn add(self, rhs: Self) -> Self::Output {
        Text::String(self.as_string() + &rhs.as_string())
    }
}

impl Add for Text {
    type Output = Text;

    fn add(self, rhs: Self) -> Self::Output {
        Text::String(self.as_string() + &rhs.as_string())
    }
}

impl Add<Text> for &Text {
    type Output = Text;

    fn add(self, rhs: Text) -> Self::Output {
        Text::String(self.as_string() + &rhs.as_string())
    }
}

impl Add<&Text> for Text {
    type Output = Text;

    fn add(self, rhs: &Text) -> Self::Output {
        Text::String(self.as_string() + &rhs.as_string())
    }
}

impl Add<&str> for Text {
    type Output = Text;

    fn add(self, rhs: &str) -> Self::Output {
        Text::String(self.as_string() + rhs)
    }
}

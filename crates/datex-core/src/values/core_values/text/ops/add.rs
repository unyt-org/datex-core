use core::ops::Add;

use crate::values::core_values::text::Text;

impl Add for &Text {
    type Output = Text;

    fn add(self, rhs: Self) -> Self::Output {
        Text(self.0.clone() + &rhs.0)
    }
}

impl Add for Text {
    type Output = Text;

    fn add(self, rhs: Self) -> Self::Output {
        Text(self.0 + &rhs.0)
    }
}

impl Add<Text> for &Text {
    type Output = Text;

    fn add(self, rhs: Text) -> Self::Output {
        Text(self.0.clone() + &rhs.0)
    }
}

impl Add<&Text> for Text {
    type Output = Text;

    fn add(self, rhs: &Text) -> Self::Output {
        Text(self.0 + &rhs.0)
    }
}

impl Add<&str> for Text {
    type Output = Text;

    fn add(self, rhs: &str) -> Self::Output {
        Text(self.0 + rhs)
    }
}

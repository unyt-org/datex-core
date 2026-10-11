use crate::{
    utils::{goat::Goat, goat_mut::GoatMut},
    values::core_values::native::DatexNative,
};

impl<'a> Goat<'a, dyn DatexNative> {
    /// Attempts to downcast the [Goat] of a dynamic [DatexNative] trait object to a [Goat] of a specific type `T`.
    /// Returns `Ok(Goat<'a, T>)` if the downcast is successful, or `Err(self)` if it fails.
    pub fn try_as<T>(self) -> Result<Goat<'a, T>, Self>
    where
        T: DatexNative + 'static,
    {
        // first check if can downcast to T
        if self.as_any().downcast_ref::<T>().is_some() {
            Ok(self.map(|v| v.as_any().downcast_ref::<T>().unwrap()))
        } else {
            Err(self)
        }
    }
}

impl<'a> GoatMut<'a, dyn DatexNative> {
    /// Attempts to downcast the [GoatMut] of a dynamic [DatexNative] trait object to a [GoatMut] of a specific type `T`.
    /// Returns `Ok(GoatMut<'a, T>)` if the downcast is successful, or `Err(self)` if it fails.
    pub fn try_as_mut<T>(self) -> Result<GoatMut<'a, T>, Self>
    where
        T: DatexNative + 'static,
    {
        // first check if can downcast to T
        if self.as_any().downcast_ref::<T>().is_some() {
            Ok(self.map(|v| v.as_any_mut().downcast_mut::<T>().unwrap()))
        } else {
            Err(self)
        }
    }
}

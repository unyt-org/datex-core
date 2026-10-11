use core::{
    cell::RefMut,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// A goat can be a Ref or a borrowed value
pub enum GoatMut<'a, T: ?Sized> {
    Ref(RefMut<'a, T>),
    Borrowed(&'a mut T),
}

impl<'a, T: ?Sized> GoatMut<'a, T> {
    /// equivalent to [RefMut::map] function
    pub fn map<U: ?Sized, F>(self, f: F) -> GoatMut<'a, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        match self {
            GoatMut::Ref(r) => GoatMut::Ref(RefMut::map(r, f)),
            GoatMut::Borrowed(b) => GoatMut::Borrowed(f(b)),
        }
    }

    /// equivalent to [RefMut::filter_map] function
    pub fn filter_map<U: ?Sized, F>(self, f: F) -> Option<GoatMut<'a, U>>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        match self {
            GoatMut::Ref(r) => RefMut::filter_map(r, f).map(GoatMut::Ref).ok(),
            GoatMut::Borrowed(b) => f(b).map(GoatMut::Borrowed),
        }
    }

    pub fn try_filter_map<U: ?Sized, F>(
        self,
        f: F,
    ) -> Result<GoatMut<'a, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        match self {
            GoatMut::Ref(r) => RefMut::filter_map(r, f)
                .map(GoatMut::Ref)
                .map_err(GoatMut::Ref),
            GoatMut::Borrowed(b) => {
                let ptr: *mut T = b;
                // SAFETY: `F` is higher-ranked over the borrow's lifetime
                // (`for<'x> FnOnce(&'x mut T) -> Option<&'x mut U>`), so `f`
                // cannot keep the reference beyond the call. If it returns
                // `None`, the reborrow is dead and we may create a new one.
                match f(unsafe { &mut *ptr }) {
                    Some(u) => Ok(GoatMut::Borrowed(u)),
                    None => Err(GoatMut::Borrowed(unsafe { &mut *ptr })),
                }
            }
        }
    }
}

impl<T: ?Sized> Debug for GoatMut<'_, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GoatMut::Ref(r) => r.fmt(f),
            GoatMut::Borrowed(b) => b.fmt(f),
        }
    }
}

impl<'a, T: ?Sized> From<RefMut<'a, T>> for GoatMut<'a, T> {
    fn from(r: RefMut<'a, T>) -> Self {
        GoatMut::Ref(r)
    }
}
impl<'a, T: ?Sized> From<&'a mut T> for GoatMut<'a, T> {
    fn from(b: &'a mut T) -> Self {
        GoatMut::Borrowed(b)
    }
}

impl<T: ?Sized> Deref for GoatMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            GoatMut::Ref(r) => r.deref(),
            GoatMut::Borrowed(b) => b,
        }
    }
}
impl<T: ?Sized> DerefMut for GoatMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            GoatMut::Ref(r) => r.deref_mut(),
            GoatMut::Borrowed(b) => b,
        }
    }
}

impl<T: ?Sized> AsRef<T> for GoatMut<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            GoatMut::Ref(r) => r.deref(),
            GoatMut::Borrowed(b) => b,
        }
    }
}

impl<T: Clone> GoatMut<'_, T> {
    pub fn into_owned(self) -> T {
        match self {
            GoatMut::Ref(r) => r.clone(),
            GoatMut::Borrowed(b) => b.clone(),
        }
    }
}

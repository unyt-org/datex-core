use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub struct SharedLazyValue<T>(Rc<RefCell<Option<T>>>);
impl<T> SharedLazyValue<T> {
    /// Creates a new uninitialized SharedLazyValue
    pub fn new() -> Self {
        SharedLazyValue(Rc::new(RefCell::new(None)))
    }

    /// Sets the value. Overwrites any existing value.
    pub fn set(&self, value: T) {
        *self.0.borrow_mut() = Some(value);
    }

    /// Gets a reference to the value. Panics if the value is not initialized.
    pub fn get<'a>(&'a self) -> Ref<'a, T> {
        Ref::map(self.0.borrow(), |opt| {
            opt.as_ref().expect("Value not initialized")
        })
    }
    pub fn get_mut<'a>(&'a self) -> RefMut<'a, T> {
        RefMut::map(self.0.borrow_mut(), |opt| {
            opt.as_mut().expect("Value not initialized")
        })
    }

    /// Tries to get a reference to the value. Returns None if the value is not initialized.
    pub fn try_get<'a>(&'a self) -> Ref<'a, Option<T>> {
        self.0.borrow()
    }

    pub fn try_get_mut<'a>(&'a self) -> RefMut<'a, Option<T>> {
        self.0.borrow_mut()
    }
}

impl<T> Clone for SharedLazyValue<T> {
    fn clone(&self) -> Self {
        SharedLazyValue(self.0.clone())
    }
}

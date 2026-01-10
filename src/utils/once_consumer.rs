/// A utility struct that allows consuming a value exactly once.
#[derive(Debug)]
pub struct OnceConsumer<T>(Option<T>);

impl<T> OnceConsumer<T> {
    pub fn new(value: T) -> Self {
        OnceConsumer(Some(value))
    }

    /// Consumes the value, returning it. Panics if the value has already been consumed.
    /// The caller must ensure that this method was not called before.
    pub fn consume(&mut self) -> T {
        self.0.take().expect("Value has already been consumed")
    }
}

impl<T> From<Option<T>> for OnceConsumer<T> {
    fn from(opt: Option<T>) -> Self {
        OnceConsumer(opt)
    }
}
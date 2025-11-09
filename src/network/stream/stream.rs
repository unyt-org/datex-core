use crate::stdlib::{cell::RefCell, collections::VecDeque, rc::Rc};

pub trait Stream<T> {
    fn push(&mut self, item: T);
    fn next(&mut self) -> Option<T>;
    fn is_empty(&self) -> bool;
    fn has_next(&self) -> bool {
        !self.is_empty()
    }
    fn len(&self) -> usize;
    fn end(&mut self);
    fn is_ended(&self) -> bool;
    fn to_ref_cell(self) -> Rc<RefCell<Self>>
    where
        Self: Sized + 'static,
    {
        Rc::new(RefCell::new(self))
    }
}

pub struct QueuingStream<T> {
    buffer: VecDeque<T>,
    ended: bool,
}
impl<T> Default for QueuingStream<T> {
    fn default() -> Self {
        Self {
            buffer: VecDeque::new(),
            ended: false,
        }
    }
}
impl<T> QueuingStream<T> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self {
            buffer: VecDeque::from(vec),
            ended: false,
        }
    }
}
impl<T> Stream<T> for QueuingStream<T> {
    fn push(&mut self, item: T) {
        self.buffer.push_back(item);
    }

    fn next(&mut self) -> Option<T> {
        self.buffer.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn end(&mut self) {
        self.ended = true;
    }

    fn is_ended(&self) -> bool {
        self.ended
    }
}

pub struct SamplingStream<T> {
    buffer: Option<T>,
    ended: bool,
    counter: usize,
}
impl<T> Default for SamplingStream<T> {
    fn default() -> Self {
        Self {
            buffer: None,
            ended: false,
            counter: 0,
        }
    }
}
impl<T> SamplingStream<T> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn new_with_value(value: T) -> Self {
        Self {
            buffer: Some(value),
            ended: false,
            counter: 1,
        }
    }
    pub fn counter(&self) -> usize {
        self.counter
    }
}
impl<T> Stream<T> for SamplingStream<T> {
    fn push(&mut self, item: T) {
        self.counter += 1;
        self.buffer = Some(item);
    }

    fn next(&mut self) -> Option<T> {
        self.buffer.take()
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_none()
    }

    fn len(&self) -> usize {
        self.buffer.as_ref().map(|_| 1).unwrap_or(0)
    }

    fn end(&mut self) {
        self.ended = true;
    }

    fn is_ended(&self) -> bool {
        self.ended
    }
}

#[cfg(test)]
mod tests {
    use crate::network::stream::Stream::{QueuingStream, SamplingStream};

    use super::*;

    #[test]
    fn test_sampling_stream() {
        let mut stream = SamplingStream {
            buffer: None,
            ended: false,
            counter: 0,
        };

        stream.push(1);
        assert_eq!(stream.len(), 1);
        assert_eq!(stream.next(), Some(1));
        assert_eq!(stream.len(), 0);
        assert_eq!(stream.next(), None);

        stream.push(2);
        stream.push(3);
        assert_eq!(stream.len(), 1);
        assert_eq!(stream.next(), Some(3));
        assert_eq!(stream.len(), 0);
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_queuing_stream() {
        let mut stream = QueuingStream {
            buffer: VecDeque::new(),
            ended: false,
        };

        stream.push(1);
        stream.push(2);
        stream.push(3);
        assert_eq!(stream.len(), 3);
        assert_eq!(stream.next(), Some(1));
        assert_eq!(stream.len(), 2);
        assert_eq!(stream.next(), Some(2));
        assert_eq!(stream.len(), 1);
        assert_eq!(stream.next(), Some(3));
        assert_eq!(stream.len(), 0);
        assert_eq!(stream.next(), None);
    }
}

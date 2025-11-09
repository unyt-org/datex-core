use crate::stdlib::pin::Pin;

use futures::{Stream, channel};
use futures_core::FusedStream;

#[derive(Debug)]
pub enum StreamError {
    Closed,
    SendError,
    RecvError,
}

pub struct DatexUnboundedSender<T> {
    tx: channel::mpsc::UnboundedSender<T>,
}
impl<T> DatexUnboundedSender<T> {
    pub fn send<M: Into<T>>(&self, item: M) -> Result<(), StreamError> {
        self.tx
            .unbounded_send(item.into())
            .map_err(|_| StreamError::SendError)
    }
    pub fn close(&self) {
        self.tx.close_channel();
    }
}

pub struct DatexStream<T> {
    rx: channel::mpsc::UnboundedReceiver<T>,
}
impl<T> DatexStream<T> {
    pub fn is_closed(&self) -> bool {
        self.rx.is_terminated()
    }
    pub fn close(&mut self) {
        self.rx.close();
    }
    // pub fn next(&mut self) -> Option<T> {
    //     self.rx.try_next().ok().flatten()
    // }
}
impl<T> Stream for DatexStream<T> {
    type Item = T;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().rx).poll_next(cx)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rx.size_hint()
    }
}

impl<T> Iterator for DatexStream<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.rx.try_next().ok().flatten()
    }
}

pub fn unbounded<T>() -> (DatexUnboundedSender<T>, DatexStream<T>) {
    let (tx, rx) = channel::mpsc::unbounded();
    (DatexUnboundedSender { tx }, DatexStream { rx })
}

#[cfg(test)]
mod tests {
    use json_syntax::print;

    use super::*;

    #[tokio::test]
    async fn test_datex_stream() {
        let (tx, mut stream) = unbounded::<u8>();

        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();

        for i in stream {
            println!("Received: {}", i);
        }

        // assert_eq!(stream.next(), Some(1));
        // assert_eq!(stream.next(), Some(2));
        // assert_eq!(stream.next(), Some(3));
        // assert_eq!(stream.next(), None);
    }
}

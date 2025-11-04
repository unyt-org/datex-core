use core::prelude::rust_2024::*;
use cfg_if::cfg_if;
use futures_util::{FutureExt, SinkExt, StreamExt};
use log::info;
use core::cell::RefCell;
use core::future::Future;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use core::clone::Clone;

type LocalPanicChannel =
    Option<(
        Option<RefCell<UnboundedSender<Signal>>>,
        Option<UnboundedReceiver<Signal>>,
    )>;

#[cfg_attr(not(feature = "embassy_runtime"), thread_local)]
static mut LOCAL_PANIC_CHANNEL: LocalPanicChannel = None;


enum Signal {
    Panic(String),
    Exit,
}

/// Creates an async execution context in which `spawn_local` or `spawn_with_panic_notify` can be used.
/// When a panic occurs in a background task spawned with `spawn_with_panic_notify`, the panic will
/// be propagated to the main task and the execution will be stopped.
///
/// Example usage:
/// ```rust
/// use datex_core::run_async;
/// use datex_core::task::{spawn_with_panic_notify_default};
///
/// async fn example() {
///     run_async! {
///         tokio::time::sleep(core::time::Duration::from_secs(1)).await;
///         spawn_with_panic_notify_default(async {
///             // Simulate a panic
///             core::panic!("This is a test panic");
///        });
///     }
/// }
/// ```
#[macro_export]
macro_rules! run_async {
    ($($body:tt)*) => {{
        datex_core::task::init_panic_notify();

        tokio::task::LocalSet::new()
            .run_until(async move {
                let res = (async move { $($body)* }).await;
                datex_core::task::close_panic_notify().await;
                datex_core::task::unwind_local_spawn_panics().await;
                res
            }).await
    }}
}

/// Spawns a thread that runs an async block using the Tokio runtime.
/// The behavior is similar to `run_async! {}`, with the only difference being that
/// it runs in a separate thread.
#[macro_export]
macro_rules! run_async_thread {
    ($($body:tt)*) => {{
        thread::spawn(move || {
            // tokio runtime setup
            let runtime = tokio::runtime::Runtime::new().unwrap();

            // Run an async block using the runtime
            runtime.block_on(async {
                run_async! {
                    $($body)*
                }
            });
        })
    }}
}

pub fn init_panic_notify() {
    let (tx, rx) = create_unbounded_channel::<Signal>();
    unsafe {
        let channel = &mut LOCAL_PANIC_CHANNEL;
        if channel.is_none() {
            *channel = Some((Some(RefCell::new(tx)), Some(rx)));
        } else {
            core::panic!("Panic channel already initialized");
        }
    }
}

#[allow(clippy::await_holding_refcell_ref)]
pub async fn close_panic_notify() {
    unsafe {
        if let Some((tx, _)) = &mut LOCAL_PANIC_CHANNEL {
            tx
                .take()
                .clone()
                .unwrap()
                .borrow_mut()
                .send(Signal::Exit)
                .await
                .expect("Failed to send exit signal");
        } else {
            core::panic!("Panic channel not initialized");
        }
    }
}

pub async fn unwind_local_spawn_panics() {
    unsafe {
        if let Some((_, rx)) = &mut LOCAL_PANIC_CHANNEL {
            let mut rx = rx.take().unwrap();
            info!("Waiting for local spawn panics...");
            if let Some(panic_msg) = rx.next().await {
                match panic_msg {
                    Signal::Exit => {}
                    Signal::Panic(panic_msg) => {
                        core::panic!("Panic in local spawn: {panic_msg}");
                    }
                }
            }
        } else {
            core::panic!("Panic channel not initialized");
        }
    }
}

#[allow(clippy::await_holding_refcell_ref)]
async fn send_panic(panic: String) {
    unsafe {
        if let Some((tx, _)) = &LOCAL_PANIC_CHANNEL {
            tx.clone()
                .expect("Panic channel not initialized")
                .borrow_mut()
                .send(Signal::Panic(panic))
                .await
                .expect("Failed to send panic");
        } else {
            core::panic!("Panic channel not initialized");
        }
    }
}
#[cfg(feature = "embassy_runtime")]
pub fn spawn_with_panic_notify<S>(async_context: &AsyncContext, spawn_token: embassy_executor::SpawnToken<S>) {
    async_context.spawner.spawn(spawn_token).expect("Spawn Error");
}

#[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
pub fn spawn_with_panic_notify<F>(_async_context: &AsyncContext, fut: F)
where
    F: Future<Output = ()> + 'static,
{
    spawn_with_panic_notify_default(fut);
}

#[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
pub fn spawn_with_panic_notify_default<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    spawn_local(async {
        let result = core::panic::AssertUnwindSafe(fut).catch_unwind().await;
        if let Err(err) = result {
            let panic_msg = if let Some(s) = err.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = err.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic type".to_string()
            };
            send_panic(panic_msg).await;
        }
    });
}


cfg_if! {
    if #[cfg(feature = "tokio_runtime")] {
        pub async fn timeout<T>(
            duration: core::time::Duration,
            fut: impl Future<Output = T>,
        ) -> Result<T, ()> {
            tokio::time::timeout(duration, fut)
                .await
                .map_err(|_| ())
        }

        pub fn spawn_local<F>(fut: F)-> tokio::task::JoinHandle<()>
        where
            F: Future<Output = ()> + 'static,
        {
            tokio::task::spawn_local(fut)
        }
        pub fn spawn<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
        where
            F: Future<Output = ()> + Send + 'static,
        {
            tokio::spawn(fut)
        }
        pub fn spawn_blocking<F, R>(f: F) -> tokio::task::JoinHandle<R>
        where
            F: FnOnce() -> R + Send + 'static,
            R: Send + 'static,
        {
            tokio::task::spawn_blocking(f)
        }
        pub async fn sleep(dur: core::time::Duration) {
            tokio::time::sleep(dur).await;
        }

    }

    else if #[cfg(feature = "wasm_runtime")] {
        use futures::future;

        pub async fn timeout<T>(
            duration: core::time::Duration,
            fut: impl Future<Output = T>,
        ) -> Result<T, ()> {
            let timeout_fut = sleep(duration);
            futures::pin_mut!(fut);
            futures::pin_mut!(timeout_fut);

            match future::select(fut, timeout_fut).await {
                future::Either::Left((res, _)) => Ok(res),
                future::Either::Right(_) => Err(()),
            }
        }
        pub async fn sleep(dur: core::time::Duration) {
            gloo_timers::future::sleep(dur).await;
        }

        pub fn spawn_local<F>(fut: F)
        where
            F: core::future::Future<Output = ()> + 'static,
        {
            wasm_bindgen_futures::spawn_local(fut);
        }
        pub fn spawn<F>(fut: F)
        where
            F: core::future::Future<Output = ()> + 'static,
        {
            wasm_bindgen_futures::spawn_local(fut);
        }
        pub fn spawn_blocking<F>(_fut: F) -> !
        where
            F: core::future::Future + 'static,
        {
            core::panic!("`spawn_blocking` is not supported in the wasm runtime.");
        }
    }

    else if #[cfg(feature = "embassy_runtime")] {
        use embassy_time::{Duration, Timer};
        use embassy_futures::select::select;
        use embassy_futures::select::Either;

        pub async fn sleep(dur: core::time::Duration) {
            let emb_dur = Duration::from_millis(dur.as_millis() as u64);
            Timer::after(emb_dur).await;
        }

        pub async fn timeout<T>(
            duration: core::time::Duration,
            fut: impl Future<Output = T>,
        ) -> Result<T, ()> {
            let emb_dur = Duration::from_millis(duration.as_millis() as u64);
            let timeout = Timer::after(emb_dur);

            match select(fut, timeout).await {
                Either::First(t) => Ok(t),
                Either::Second(_) => Err(()),
            }
        }

    }
    else {
        compile_error!("Unsupported runtime. Please enable either 'tokio_runtime', 'embassy_runtime' or 'wasm_runtime' feature.");
    }
}


#[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
use futures::channel::mpsc::{UnboundedReceiver as _UnboundedReceiver, UnboundedSender as _UnboundedSender, Receiver as _Receiver, Sender as _Sender};
#[cfg(feature = "embassy_runtime")]
pub use async_unsync::bounded::{Receiver as _Receiver, Sender as _Sender};
#[cfg(feature = "embassy_runtime")]
pub use async_unsync::unbounded::{UnboundedReceiver as _UnboundedReceiver, UnboundedSender as _UnboundedSender};
use datex_core::runtime::AsyncContext;

#[derive(Debug)]
pub struct Receiver<T>(_Receiver<T>);
impl<T> Receiver<T> {
    pub fn new(receiver: _Receiver<T>) -> Self {
        Receiver(receiver)
    }

    pub async fn next(&mut self) -> Option<T> {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        { self.0.next().await}
        #[cfg(feature = "embassy_runtime")]
        { self.0.recv().await }
    }
}

#[derive(Debug)]
pub struct UnboundedReceiver<T>(_UnboundedReceiver<T>);
impl<T> UnboundedReceiver<T> {
    pub fn new(receiver: _UnboundedReceiver<T>) -> Self {
        UnboundedReceiver(receiver)
    }
    pub async fn next(&mut self) -> Option<T> {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        { self.0.next().await}
        #[cfg(feature = "embassy_runtime")]
        { self.0.recv().await }
    }

}

#[derive(Debug)]
pub struct Sender<T>(_Sender<T>);

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender(self.0.clone())
    }
}
impl<T> Sender<T> {
    pub fn new(sender: _Sender<T>) -> Self {
        Sender(sender)
    }

    pub fn start_send(&mut self, item: T) -> Result<(), ()> {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        {self.0.start_send(item).map_err(|_| ())}
        #[cfg(feature = "embassy_runtime")]
        {self.0.try_send(item).map_err(|_| ())}
    }

    pub async fn send(&mut self, item: T) -> Result<(), ()> {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        {self.0.send(item).await.map_err(|_| ()).map(|_| ())}
        #[cfg(feature = "embassy_runtime")]
        {self.0.send(item).await.map(|_| ()).map_err(|_| ())}
    }

    pub fn close_channel(&mut self) {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        {self.0.close_channel();}
        #[cfg(feature = "embassy_runtime")]
        {}
    }
}

#[derive(Debug)]
pub struct UnboundedSender<T>(_UnboundedSender<T>);

// FIXME: derive Clone?
impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        UnboundedSender(self.0.clone())
    }
}

impl<T> UnboundedSender<T> {
    pub fn new(sender: _UnboundedSender<T>) -> Self {
        UnboundedSender(sender)
    }

    pub fn start_send(&mut self, item: T) -> Result<(), ()> {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        {self.0.start_send(item).map_err(|_| ())}
        #[cfg(feature = "embassy_runtime")]
        {self.0.send(item).map_err(|_| ())}
    }

    pub async fn send(&mut self, item: T) -> Result<(), ()> {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        {self.0.send(item).await.map_err(|_| ()).map(|_| ())}
        #[cfg(feature = "embassy_runtime")]
        {self.0.send(item).map(|_| ()).map_err(|_| ())}
    }

    pub fn close_channel(&self) {
        #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))]
        {self.0.close_channel();}
        #[cfg(feature = "embassy_runtime")]
        {}
    }
}



cfg_if! {
    if #[cfg(any(feature = "tokio_runtime", feature = "wasm_runtime"))] {
        pub fn create_bounded_channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
            let (sender, receiver) = futures::channel::mpsc::channel::<T>(capacity);
            (Sender::new(sender), Receiver::new(receiver))
        }
        pub fn create_unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
            let (sender, receiver) = futures::channel::mpsc::unbounded::<T>();
            (UnboundedSender::new(sender), UnboundedReceiver::new(receiver))
        }
    }
    else if #[cfg(feature = "embassy_runtime")] {
        pub fn create_bounded_channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
            let (sender, receiver) = async_unsync::bounded::channel::<T>(capacity).into_split();
            (Sender::new(sender), Receiver::new(receiver))
        }
         pub fn create_unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
            let (sender, receiver) = async_unsync::unbounded::channel::<T>().into_split();
            (UnboundedSender::new(sender), UnboundedReceiver::new(receiver))
        }
    }
    else {
        compile_error!("Unsupported runtime. Please enable either 'tokio_runtime', 'embassy_runtime' or 'wasm_runtime' feature.");
    }
}
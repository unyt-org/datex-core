use cfg_if::cfg_if;
use futures::channel::mpsc;
use futures_util::{FutureExt, SinkExt, StreamExt};
use log::info;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

type LocalPanicChannel = Rc<
    RefCell<
        Option<(
            Option<RefCell<mpsc::UnboundedSender<Signal>>>,
            Option<mpsc::UnboundedReceiver<Signal>>,
        )>,
    >,
>;
thread_local! {
    static LOCAL_PANIC_CHANNEL: LocalPanicChannel = Rc::new(RefCell::new(None));
}

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
/// use datex_core::task::spawn_with_panic_notify;
///
/// async fn example() {
///     run_async! {
///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
///         spawn_with_panic_notify(async {
///             // Simulate a panic
///             panic!("This is a test panic");
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
    let (tx, rx) = mpsc::unbounded::<Signal>();
    LOCAL_PANIC_CHANNEL
        .try_with(|channel| {
            let mut channel = channel.borrow_mut();
            if channel.is_none() {
                *channel = Some((Some(RefCell::new(tx)), Some(rx)));
            } else {
                panic!("Panic channel already initialized");
            }
        })
        .expect("Failed to initialize panic channel");
}

#[allow(clippy::await_holding_refcell_ref)]
pub async fn close_panic_notify() {
    LOCAL_PANIC_CHANNEL
        .with(|channel| {
            let channel = channel.clone();
            let mut channel = channel.borrow_mut();
            if let Some((tx, _)) = &mut *channel {
                tx.take()
            } else {
                panic!("Panic channel not initialized");
            }
        })
        .expect("Failed to access panic channel")
        .clone()
        .borrow_mut()
        .send(Signal::Exit)
        .await
        .expect("Failed to send exit signal");
}

pub async fn unwind_local_spawn_panics() {
    let mut rx = LOCAL_PANIC_CHANNEL
        .with(|channel| {
            let channel = channel.clone();
            let mut channel = channel.borrow_mut();
            if let Some((_, rx)) = &mut *channel {
                rx.take()
            } else {
                panic!("Panic channel not initialized");
            }
        })
        .expect("Failed to access panic channel");
    info!("Waiting for local spawn panics...");
    if let Some(panic_msg) = rx.next().await {
        match panic_msg {
            Signal::Exit => {}
            Signal::Panic(panic_msg) => {
                panic!("Panic in local spawn: {panic_msg}");
            }
        }
    }
}

#[allow(clippy::await_holding_refcell_ref)]
async fn send_panic(panic: String) {
    LOCAL_PANIC_CHANNEL
        .try_with(|channel| {
            let channel = channel.clone();
            let channel = channel.borrow_mut();
            if let Some((tx, _)) = &*channel {
                tx.clone().expect("Panic channel not initialized")
            } else {
                panic!("Panic channel not initialized");
            }
        })
        .expect("Failed to access panic channel")
        .borrow_mut()
        .send(Signal::Panic(panic))
        .await
        .expect("Failed to send panic");
}

pub fn spawn_with_panic_notify<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    spawn_local(async {
        let result = std::panic::AssertUnwindSafe(fut).catch_unwind().await;
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
        pub fn timeout<F>(duration: std::time::Duration, fut: F) -> tokio::time::Timeout<F::IntoFuture>
        where
            F: std::future::IntoFuture,
        {
            tokio::time::timeout(duration, fut)
        }

        pub fn spawn_local<F>(fut: F)-> tokio::task::JoinHandle<()>
        where
            F: std::future::Future<Output = ()> + 'static,
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
        pub async fn sleep(dur: std::time::Duration) {
            tokio::time::sleep(dur).await;
        }

    } else if #[cfg(feature = "wasm_runtime")] {
        use futures::future;

        pub async fn timeout<F, T>(
            duration: std::time::Duration,
            fut: F,
        ) -> Result<T, &'static str>
        where
            F: std::future::Future<Output = T>,
        {
            let timeout_fut = sleep(duration);
            futures::pin_mut!(fut);
            futures::pin_mut!(timeout_fut);

            match future::select(fut, timeout_fut).await {
                future::Either::Left((res, _)) => Ok(res),
                future::Either::Right(_) => Err("timed out"),
            }
        }
        pub async fn sleep(dur: std::time::Duration) {
            gloo_timers::future::sleep(dur).await;
        }

        pub fn spawn_local<F>(fut: F)
        where
            F: std::future::Future<Output = ()> + 'static,
        {
            wasm_bindgen_futures::spawn_local(fut);
        }
        pub fn spawn<F>(fut: F)
        where
            F: std::future::Future<Output = ()> + 'static,
        {
            wasm_bindgen_futures::spawn_local(fut);
        }
        pub fn spawn_blocking<F>(_fut: F) -> !
        where
            F: std::future::Future + 'static,
        {
            panic!("`spawn_blocking` is not supported in the wasm runtime.");
        }
    } else {
        compile_error!("Unsupported runtime. Please enable either 'tokio_runtime' or 'wasm_runtime' feature.");
    }
}

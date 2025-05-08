use cfg_if::cfg_if;
use futures::channel::mpsc;
use futures_util::{FutureExt, SinkExt, StreamExt};
use log::{error, info};
use std::future::Future;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    static ref LOCAL_PANIC_CHANNEL: Mutex<Option<(
        Arc<Mutex<mpsc::UnboundedSender<Signal>>>,
        Option<mpsc::UnboundedReceiver<Signal>>,
    )>> = Mutex::new(None);
}

enum Signal {
    Panic(String),
    Exit,
}

#[macro_export]
macro_rules! run_async {
    ($($body:tt)*) => {{
        datex_core::task::init_panic_notify();

        task::LocalSet::new()
            .run_until(async move {
                datex_core::task::spawn_with_panic_notify(async move {
                    (async move { $($body)* }).await;
                    datex_core::task::close_panic_notify().await;
                });
                datex_core::task::unwind_local_spawn_panics().await;
            }).await;
    }}
}

pub fn init_panic_notify() {
    let (tx, rx) = mpsc::unbounded::<Signal>();
    let mut local_panic_channel = LOCAL_PANIC_CHANNEL.lock().unwrap();

    *local_panic_channel = Some((Arc::new(Mutex::new(tx)), Some(rx)));
}

pub async fn close_panic_notify() {
    let mut local_panic_channel = LOCAL_PANIC_CHANNEL.lock().unwrap();
    if let Some((tx, _)) = &mut *local_panic_channel {
        let mut tx = tx.lock().unwrap();
        tx.send(Signal::Exit).await.unwrap();
    }
}

pub async fn unwind_local_spawn_panics() {
    let mut rx = {
        let mut local_panic_channel = LOCAL_PANIC_CHANNEL.lock().unwrap();
        if let Some((_, ref mut rx)) = &mut *local_panic_channel {
            rx.take().unwrap()
        } else {
            panic!("Panic channel not initialized");
        }
    };
    info!("Waiting for local spawn panics...");
    if let Some(panic_msg) = rx.next().await {
        match panic_msg {
            Signal::Exit => {
                info!("Exiting local spawn panics");
            }
            Signal::Panic(panic_msg) => {
                panic!("Panic in local spawn: {panic_msg}");
            }
        }
    }
}
fn get_tx() -> Arc<Mutex<mpsc::UnboundedSender<Signal>>> {
    let local_panic_channel = LOCAL_PANIC_CHANNEL.lock().unwrap();
    if let Some((tx, _)) = &*local_panic_channel {
        tx.clone()
    } else {
        panic!("Panic channel not initialized");
    }
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
            let tx = get_tx();
            let tx = tx.lock();
            tx.unwrap().send(Signal::Panic(panic_msg)).await.unwrap();
            error!("exited");
        }
    });
}
cfg_if! {
    if #[cfg(feature = "tokio_runtime")] {

        pub fn spawn_local<F>(fut: F)
        where
            F: std::future::Future<Output = ()> + 'static,
        {
            tokio::task::spawn_local(fut);
        }
        pub fn spawn<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
        where
            F: std::future::Future<Output = ()> + Send + 'static,
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

    } else if #[cfg(feature = "wasm_runtime")] {
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

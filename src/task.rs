use cfg_if::cfg_if;

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

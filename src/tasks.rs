use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "tokio-runtime")] {
        pub fn spawn_local<F>(fut: F)
        where
            F: std::future::Future<Output = ()> + 'static,
        {
            tokio::task::spawn_local(fut);
        }
    } else if #[cfg(feature = "wasm-runtime")] {
        pub fn spawn_local<F>(fut: F)
        where
            F: std::future::Future<Output = ()> + 'static,
        {
            wasm_bindgen_futures::spawn_local(fut);
        }
    } else {
        compile_error!("Unsupported runtime. Please enable either 'tokio-runtime' or 'wasm-runtime' feature.");
    }
}

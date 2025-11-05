use cfg_if::cfg_if;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

static INIT: AtomicBool = AtomicBool::new(false);

/// Initializes the logger with debug mode, logging all messages including debug messages.
pub fn init_logger_debug() {
    if !INIT.swap(true, Ordering::SeqCst) {
        init(true);
    }
}

/// Initializes the logger with default mode, only logging errors and above.
pub fn init_logger() {
    if !INIT.swap(true, Ordering::SeqCst) {
        init(false);
    }
}

cfg_if! {
    if #[cfg(feature = "flexi_logger")] {
        use flexi_logger;
        fn init(debug: bool) {
            let env = if debug {
                "datex_core=trace,r#mod=trace"
            } else {
                "datex_core=error,r#mod=error"
            };
            flexi_logger::Logger::try_with_env_or_str(env).expect("Failed to initialize logger")
                .start()
                .expect("Failed to start logger");
        }
    }

    else if #[cfg(feature = "wasm_logger")]
    {
        fn init(debug: bool) {
            use log::Level;
            use console_error_panic_hook;

            console_log::init_with_level(
                if debug {
                    Level::Debug
                } else {
                    Level::Error
                },
            ).expect("Failed to initialize logger");
            console_error_panic_hook::set_once();
        }
    }

    else if #[cfg(feature = "env_logger")] {
        use env_logger;
        fn init(debug: bool) {
            env_logger::init();
            info!("Logger initialized! (Using env_logger)");
        }
    }

    else if #[cfg(feature = "esp_logger")] {
        fn init(debug: bool) {
            if debug {
                esp_println::logger::init_logger(log::LevelFilter::Debug);
            }
            else {
                esp_println::logger::init_logger(log::LevelFilter::Info);
            }
        }
    }

    else {
        fn init(debug: bool) {
            #[cfg(feature = "std")]
            {println!("No logger enabled. Logs will not be recorded.");}
        }
    }
}

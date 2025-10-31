use crate::stdlib::sync::Once;

use cfg_if::cfg_if;

static INIT: Once = Once::new();

/// Initializes the logger with debug mode, logging all messages including debug messages.
pub fn init_logger_debug() {
    // TODO: nostd
    INIT.call_once(|| {
        init(true);
    });
}

/// Initializes the logger with default mode, only logging errors and above.
pub fn init_logger() {
    INIT.call_once(|| {
        init(false);
    });
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
        use esp_idf_svc::log::EspLogger;
        fn init(debug: bool) {
            EspLogger::initialize_default();
        }
    }

    else {
        fn init(debug: bool) {
            println!("No logger enabled. Logs will not be recorded.");
        }
    }
}

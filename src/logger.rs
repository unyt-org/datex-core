use std::sync::Once;

use cfg_if::cfg_if;
use log::info;

cfg_if! {
    if #[cfg(feature = "debug")] {
        const LOG_LEVEL: &str = "debug";
        const LOG_ENV: &str = "datex_core=trace,r#mod=trace,matchbox_socket=trace";
    } else {
        const LOG_LEVEL: &str = "info";
        const LOG_ENV: &str = "datex_core=info,matchbox_socket=info";
    }
}
static INIT: Once = Once::new();

pub fn init_logger() {
    INIT.call_once(|| {
        init();
    });
}

cfg_if! {

    if #[cfg(feature = "flexi_logger")] {
        use flexi_logger;
        fn init() {
            flexi_logger::Logger::try_with_env_or_str(LOG_ENV).expect("Failed to initialize logger")
                .start()
                .expect("Failed to start logger");
            info!("Logger initialized! (Using flexi_logger) {LOG_ENV}");
        }
    }

    else if #[cfg(feature = "wasm_logger")]
    {
        fn init() {
            use log::Level;
            console_log::init_with_level(
                if LOG_LEVEL == "debug" {
                    Level::Debug
                } else {
                    Level::Info
                },
            ).expect("Failed to initialize logger");
            info!("Logger initialized! (Using wasm_logger)");
        }
    }

    else if #[cfg(feature = "env_logger")] {
        use env_logger;
        fn init() {
            env_logger::init();
            info!("Logger initialized! (Using env_logger)");
        }
    }

    else if #[cfg(feature = "esp_logger")] {
        use esp_idf_svc::log::EspLogger;
        fn init() {
            EspLogger::initialize_default();
        }
    }

    else {
        fn init() {
            println!("No logger enabled. Logs will not be recorded.");
        }
    }
}

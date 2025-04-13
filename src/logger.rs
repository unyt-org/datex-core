use cfg_if::cfg_if;
use log::info;

cfg_if! {
    if #[cfg(feature = "debug")] {
        const LOG_LEVEL: &str = "debug";
        const LOG_ENV: &str = "datex_core=trace,r#mod=trace,matchbox_socket=info";
    } else {
        const LOG_LEVEL: &str = "info";
        const LOG_ENV: &str = "datex_core=info,matchbox_socket=info";
    }
}

cfg_if! {

    if #[cfg(feature = "flexi_logger")] {
        use flexi_logger;
        pub fn init_logger() {
            flexi_logger::Logger::try_with_env_or_str(LOG_ENV).expect("Failed to initialize logger")
                .start()
                .expect("Failed to start logger");
            info!("Logger initialized! (Using flexi_logger) {}", LOG_ENV);
        }
    }

    else if #[cfg(feature = "wasm_logger")]
    {
        pub fn init_logger() {
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
        pub fn init_logger() {
            env_logger::init();
            info!("Logger initialized! (Using env_logger)");
        }
    }

    else if #[cfg(feature = "esp_logger")] {
        use esp_idf_svc::log::EspLogger;
        pub fn init_logger() {
            EspLogger::initialize_default();
        }
    }

    else {
        pub fn init_logger() {
            println!("No logger enabled. Logs will not be recorded.");
        }
    }
}

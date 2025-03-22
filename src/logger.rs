use log::info;

#[cfg(feature = "flexi_logger")]
use flexi_logger::Logger;

#[cfg(feature = "env_logger")]
use env_logger;

cfg_if! {
    #[cfg(feature = "flexi_logger")]
    {
        Logger::try_with_env_or_str("info")
            .unwrap()
            .start()
            .unwrap();
        info!("Logger initialized! (Using flexi_logger)");
    }

    #[cfg(feature = "env_logger")]
    {
        env_logger::init();
        info!("Logger initialized! (Using env_logger)");
    }

    #[cfg(not(any(feature = "flexi_logger", feature = "env_logger")))]
    {
        // No-op: No logger is initialized
        println!("No logger enabled. Logs will not be recorded.");
    }
}

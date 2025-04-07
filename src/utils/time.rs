use cfg_if::cfg_if;

pub trait TimeTrait {
    /// Returns the current time in milliseconds since the Unix epoch.
    fn now() -> u64;
}


pub struct Time;

cfg_if! {
    if #[cfg(feature = "native_time")] {
        use std::time::{SystemTime, UNIX_EPOCH};

        impl TimeTrait for Time {
            fn now() -> u64 {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as u64
            }
        }
    }

    else if #[cfg(feature = "wasm_time")] {
        use js_sys::Date;
        
        impl TimeTrait for Time {
            fn now() -> u64 {
                Date::now() as u64
            }
        }
    }
    
    else {
        compile_error!("No time implementation available. Please enable either 'native_time' or 'wasm_time' feature.");
    }

}



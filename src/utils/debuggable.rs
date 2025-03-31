use log::log;

pub trait Debuggable {
    fn get_debug_info(&self) -> String;

    fn log_debug_info(&self) {
        let debug_info = self.get_debug_info();
        log::info!("{}", debug_info);
    }
}

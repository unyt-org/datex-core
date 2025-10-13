use datex_core::logger::init_logger_debug;
use datex_core::runtime::global_context::{GlobalContext, set_global_context};

pub fn init_global_context() {
    let global_ctx = GlobalContext::native();
    set_global_context(global_ctx);
    init_logger_debug();
}

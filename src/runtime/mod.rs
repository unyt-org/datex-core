use crate::utils::logger::LoggerContext;

mod stack;
use self::execution::execute;

mod execution;


pub struct Runtime {
    pub version: i8,
	pub ctx: LoggerContext
}

impl Runtime {
	
	pub fn new_with_ctx(ctx: LoggerContext) -> Runtime {
		return Runtime { version: 1, ctx: ctx }
	}

	pub fn new() -> Runtime {
		return Runtime { version: 1, ctx: LoggerContext { log_redirect: None } }
	}

	pub fn execute(&self, dxb: &[u8]) {
		execute(&self.ctx, dxb);
	}

}


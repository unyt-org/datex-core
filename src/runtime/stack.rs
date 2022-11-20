use crate::{utils::logger::Logger, datex_values::{Value, Error, PrimitiveValue, ValueResult}};

pub struct Stack<'a> {
	stack: Vec<Box<dyn Value>>,
	logger: &'a Logger<'a>
}

impl Stack<'_> {

	pub fn new<'a>(logger:&'a Logger<'a>) -> Stack<'a> {
		Stack { stack: Vec::new(), logger }
	}


	// custom stack operations

	pub fn print(&mut self) {
		self.logger.plain("[CURRENT STACK]");
		for item in &self.stack {
			self.logger.plain(&item.to_string())
		}
	}

	pub fn size(&mut self) -> usize {
		return self.stack.len();
	}

	pub fn push(&mut self, value: Box<dyn Value>) {
		self.stack.push(value)
	}

	pub fn pop(&mut self) -> ValueResult { 
		let value = self.stack.pop();
		if value.is_some() {
			return Ok(value.unwrap())
		}
		else {
			return Err(Error { message: "stack error".to_string() })
		}
	}

	pub fn pop_or_void(&mut self) -> Box<dyn Value> {
		let value = self.stack.pop();
		if value.is_some() {
			return value.unwrap()
		}
		else {
			return Box::new(PrimitiveValue::VOID);
		}
	}

}
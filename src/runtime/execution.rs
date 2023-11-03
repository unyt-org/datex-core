use std::cell::Cell;

use crate::{utils::logger::{LoggerContext, Logger}, datex_values::{ValueResult, PrimitiveValue, Error, Value, Type}, parser::{header::{self, has_dxb_magic_number}, body}, global::binary_codes::BinaryCode};

use super::stack::Stack;


/**
 * Converts DXB (with or without header) to DATEX Script
*/
pub fn execute(ctx: &LoggerContext, dxb:&[u8]) -> ValueResult {

	let mut body = dxb;

	// header?
	if has_dxb_magic_number(dxb) {
		let (header, _body) = header::parse_dxb_header(dxb);
		body = _body;
	}

	return execute_body(ctx, body);
}


fn execute_body(ctx: &LoggerContext, dxb_body:&[u8]) -> ValueResult {
	
	return execute_loop(ctx, dxb_body, &Cell::from(0));
}

fn execute_loop(ctx: &LoggerContext, dxb_body:&[u8], index: &Cell<usize>) -> ValueResult {

	let logger = Logger::new_for_development(ctx, "DATEX Runtime");

	let mut stack:Stack = Stack::new(&logger);

	let instruction_iterator = body::iterate_instructions(dxb_body, index);

	for instruction in instruction_iterator {
		
		logger.debug(&instruction.to_string());

		let code = instruction.code;


		let slot = instruction.slot.unwrap_or_default();
		let has_primitive_value = instruction.primitive_value.is_some();
		let has_value = instruction.value.is_some();

		let error = match code {

			BinaryCode::ADD 		=> binary_operation(code, &mut stack, &logger),
			BinaryCode::SUBTRACT 	=> binary_operation(code, &mut stack, &logger),
			BinaryCode::MULTIPLY 	=> binary_operation(code, &mut stack, &logger),
			BinaryCode::DIVIDE 		=> binary_operation(code, &mut stack, &logger),
			BinaryCode::MODULO		=> binary_operation(code, &mut stack, &logger),
			BinaryCode::POWER 		=> binary_operation(code, &mut stack, &logger),
			BinaryCode::AND 		=> binary_operation(code, &mut stack, &logger),
			BinaryCode::OR 			=> binary_operation(code, &mut stack, &logger),


			BinaryCode::CLOSE_AND_STORE => clear_stack(&mut stack, &logger),

			_ => {

				// add value to stack

				if has_value {
					let value = instruction.value.unwrap_or(Box::new(PrimitiveValue::Void));
					stack.push(value)
				}

				else if has_primitive_value {
					let primitive_value = instruction.primitive_value.unwrap_or_default();
					stack.push(Box::new(primitive_value));

				};
				None
			}
		};

		if error.is_some() {
			let error_val = error.unwrap();
			logger.error(&format!("error: {}", &error_val));
			return Err(error_val);
		}


		// enter new subscope - continue at index?
		if instruction.subscope_continue {
			let sub_result = execute_loop(ctx, dxb_body, index);

			// propagate error from subscope
			if sub_result.is_err() {
				return Err(sub_result.err().unwrap());
			}
			// push subscope result to stack
			else {
				let res = sub_result.ok().unwrap();
				logger.success(&format!("sub result: {}", res));
				stack.push(res);
			}
		}

	}

	clear_stack(&mut stack, &logger);

	return Ok(stack.pop_or_void());

}


// reset stack 
// clear from end and set final value as first stack value of new stack
fn clear_stack(stack: &mut Stack, logger:&Logger) -> Option<Error> {

	if stack.size() == 0 {return None}; // nothing to clear

	let mut current: Box<dyn Value> = stack.pop_or_void(); // get last stack value

	while stack.size() != 0 {
		let next = stack.pop_or_void();

		// type cast
		if next.is::<Type>() {
			logger.debug(&format!("cast {next} {current}"));
			let dx_type = next.downcast::<Type>();
			if dx_type.is_ok() {
				let res = current.cast(*dx_type.ok().unwrap());
 				if res.is_ok() {
					current = res.ok().unwrap();
				}
				else {return res.err()}
			}
			else {return Some(Error { message: "rust downcasting error".to_string() })}
			
		}

		// other apply
		else {
			logger.debug(&format!("apply {next} {current}"));
		}
	}

	stack.push(current);

	return None;
}

// operator handlers


fn binary_operation(code: BinaryCode, stack: &mut Stack, logger:&Logger) -> Option<Error> {
	stack.print();

	// pop 2 operands from stack
	let _s1 = stack.pop();
	if _s1.is_err() {return _s1.err()}
	let s1 = _s1.ok().unwrap();

	let _s2 = stack.pop();
	if _s2.is_err() {return _s2.err()}
	let s2 = _s2.ok().unwrap();

	// binary operation
	match s2.binary_operation(code, s1) {
		Ok(result) => {
			logger.success(&format!("binary op result: {}", result));
			stack.push(result);
			return None;
		},
		Err(err) => Some(err),
	}

}
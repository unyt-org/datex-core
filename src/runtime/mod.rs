use std::cell::Cell;

use crate::datex_values::Error;
use crate::datex_values::PrimitiveValue;
use crate::datex_values::Value;

use crate::datex_values::ValueResult;
use crate::global::binary_codes::BinaryCode;
use crate::parser::header;
use crate::parser::body;
use crate::utils::logger::Logger;
use crate::utils::logger::LoggerContext;

// lazy_static!{
// 	static ref logger:Logger = Logger::new_for_development("DATEX Runtime");
// }


/**
 * Converts DXB (with or without header) to DATEX Script
 */
pub fn execute(ctx: &LoggerContext, dxb:&[u8])  -> ValueResult {


	// header?
	if dxb[0] == 0x01 && dxb[1] == 0x64 {
		header::parse_dxb_header(dxb);
	}

	return execute_body(ctx, dxb);
}


pub fn execute_body(ctx: &LoggerContext, dxb_body:&[u8]) -> ValueResult {
	
	return execute_loop(ctx, dxb_body, &Cell::from(0));
}

type Stack = Vec<Box<dyn Value>>;

fn execute_loop(ctx: &LoggerContext, dxb_body:&[u8], index: &Cell<usize>) -> ValueResult {

	let logger = Logger::new_for_development(ctx, "DATEX Runtime");

	let mut stack:Stack = Vec::new();


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

			_ => {

				// add value to stack

				if has_value {
					let value = instruction.value.unwrap_or(Box::new(PrimitiveValue::VOID));
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
				stack.push(sub_result.ok().unwrap());
			}
		}

	}


	let result = stack.pop();
	if result.is_some() {
		return Ok(result.unwrap());
	}
	else {
		return Ok(Box::new(PrimitiveValue::VOID));
	}
	

}


fn print_stack(stack: &Stack, logger:&Logger) {
	logger.plain("[CURRENT STACK]");
	for item in stack {
		logger.plain(&item.to_string())
	}
}

fn pop_stack(stack: &mut Stack) -> Result<Box<dyn Value>, Error> {
	let value = stack.pop();
	if value.is_some() {
		return Ok(value.unwrap())
	}
	else {
		return Err(Error { message: "stack error".to_string() })
	}
}

// operator handlers


fn binary_operation(code: BinaryCode, mut stack: &mut Stack, logger:&Logger) -> Option<Error> {
	print_stack(&stack, &logger);

	// pop 2 operands from stack
	let _s1 = pop_stack(&mut stack);
	if _s1.is_err() {return _s1.err()}
	let s1 = _s1.ok().unwrap();

	let _s2 = pop_stack(&mut stack);
	if _s2.is_err() {return _s2.err()}
	let s2 = _s2.ok().unwrap();

	// binary operation
	match s2.binary_operation(code, s1) {
		Ok(result) => {
			logger.success(&format!("result: {}", result));
			stack.push(result);
			return None;
		},
		Err(err) => Some(err),
	}

}
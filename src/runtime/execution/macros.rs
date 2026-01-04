/// Yield an interrupt and get the next input
macro_rules! interrupt {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        $input.take_result()
    }};
}
pub(crate) use interrupt;

/// Yield an interrupt and get the next resolved value or None
/// expecting the next input to be a ResolvedValue variant
macro_rules! interrupt_with_maybe_value {
    ($input:expr, $arg:expr) => {{
        use crate::runtime::execution::macros::interrupt;

        let res = interrupt!($input, $arg).unwrap();
        match res {
            crate::runtime::execution::execution_loop::InterruptResult::ResolvedValue(value) => value,
            _ => unreachable!(),
        }
    }};
}
pub(crate) use interrupt_with_maybe_value;

/// Yield an interrupt and get the next resolved value
/// expecting the next input to be a ResolvedValue variant with Some value
macro_rules! interrupt_with_value {
    ($input:expr, $arg:expr) => {{
        use crate::runtime::execution::macros::interrupt_with_maybe_value;
        let maybe_value = interrupt_with_maybe_value!($input, $arg);
        if let Some(value) = maybe_value {
            value
        } else {
            unreachable!();
        }
    }};
}
pub(crate) use interrupt_with_value;

/// Unwrap a Result expression, yielding an error if it is an Err variant
/// This is similar to the `?` operator but works within generator functions
/// TODO #642: use "?" operator instead of yield_unwrap once supported in gen blocks
macro_rules! yield_unwrap {
    ($e:expr) => {{
        let res = $e;
        if let Ok(res) = res {
            res
        } else if let Err(err) = res {
            return yield Err(err.into());
        } else {
            unreachable!();
        }
    }};
}
pub(crate) use yield_unwrap;

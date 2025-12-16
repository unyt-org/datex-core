/// Get the next iteration item or break the loop
/// Useful inside loops where iteration also occurs within the loop body
/// A loop label must be provided to break from
macro_rules! next_iter {
    ($iterator:ident, $label:lifetime) => {
        match $iterator.next() {
            Some(instruction) => instruction,
            None => break $label,
        }
    };
}
pub(crate) use next_iter;


/// Intercept a specific step in an iterator.
/// Non-handled steps are yielded back to the caller.
/// The result of the matched body is returned if the step is encountered before the iterator ends.
/// If no next step is encountered, None is returned.
macro_rules! intercept_maybe_step {
    (
        $iterator:expr,
        $( $pattern:pat => $body:expr ),+ $(,)?
    ) => {        
        loop {
            let step = $iterator.next();
            if let Some(step) = step  {
                match step {
                    $(
                        $pattern => break Some($body),
                    )+
                    step => yield step,
                }
            }
            else {
                break None;
            }
        }
    };
}
pub(crate) use intercept_maybe_step;

/// Intercept a specific step in an iterator.
/// Non-handled steps are yielded back to the caller.
/// The result of the matched body is returned if the step is encountered before the iterator ends.
/// If no next step is encountered, an InvalidProgram error is yielded.
macro_rules! intercept_step {
    (
        $iterator:expr,
        $( $pattern:pat => $body:expr ),+ $(,)?
    ) => {{
        use crate::runtime::execution::errors::ExecutionError;
        use crate::runtime::execution::errors::InvalidProgramError;

        loop {
            let step = $iterator.next();
            if let Some(step) = step {
                match step {
                    $(
                        $pattern => break $body,
                    )+
                    step => yield step,
                }
            }
            else {
                return yield Err(ExecutionError::InvalidProgram(InvalidProgramError::ExpectedInstruction));
            }
        }
    }};
}
pub(crate) use intercept_step;


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
/// TODO: use "?" operator instead of yield_unwrap once supported in gen blocks
macro_rules! yield_unwrap {
    ($e:expr) => {{
        let res = $e;
        if let Ok(res) = res {
            res
        } else {
            return yield Err(res.unwrap_err().into());
        }
    }};
}
pub(crate) use yield_unwrap;

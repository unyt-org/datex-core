/// Get the next iteration item or break the loop
/// Useful inside loops where iteration also occurs within the loop body
macro_rules! next_iter {
    ($iterator:ident) => {
        match $iterator.next() {
            Some(instruction) => instruction,
            None => break,
        }
    };
}
pub(crate) use next_iter;

/// Intercept specific steps in an iterator
/// Non-handled steps are yielded back to the caller
macro_rules! intercept_steps {
    (
        $iterator:expr,
        $( $pattern:pat => $body:expr ),+ $(,)?
    ) => {
        for step in $iterator {
            match step {
                $(
                    $pattern => $body,
                )+
                step => yield step,
            }
        }
    };
}
pub(crate) use intercept_steps;

/// Intercept specific steps in an iterator
/// All steps must be handled within the macro
macro_rules! intercept_all_steps {
    (
        $iterator:expr,
        $( $pattern:pat => $body:expr ),+ $(,)?
    ) => {
        for step in $iterator {
            match step {
                $(
                    $pattern => $body,
                )+
            }
        }
    };
}
pub(crate) use intercept_all_steps;



/// Yield an interrupt and get the next input
macro_rules! interrupt {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        $input.take().unwrap()
    }};
}
pub(crate) use interrupt;

/// Yield an interrupt and get the next result value,
/// expecting the next input to be a Result variant
macro_rules! interrupt_with_result {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        let res = $input.take().unwrap();
        match res {
            InterruptProvider::Result(value) => value,
            _ => unreachable!(),
        }
    }};
}
pub(crate) use interrupt_with_result;

/// Yield an interrupt and get the next type instruction,
/// expecting the next input to be a NextTypeInstruction variant
macro_rules! interrupt_with_next_type_instruction {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        let res = $input.take().unwrap();
        match res {
            InterruptProvider::NextTypeInstruction(value) => value,
            _ => unreachable!(),
        }
    }};
}
pub(crate) use interrupt_with_next_type_instruction;

/// Unwrap a Result expression, yielding an error if it is an Err variant
/// This is similar to the `?` operator but works within generator functions
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
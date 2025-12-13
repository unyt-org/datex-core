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

/// Intercept specific steps in an iterator and perform actions
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


/// Intercept a specific step in an iterator.
/// Non-handled steps are yielded back to the caller.
/// The result of the matched body is returned if the step is encountered before the iterator ends.
macro_rules! intercept_step {
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
pub(crate) use intercept_step;

/// Intercept specific steps in an iterator
/// All steps must be handled within the macro
macro_rules! handle_steps {
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
pub(crate) use handle_steps;



/// Yield an interrupt and get the next input
macro_rules! interrupt {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        $input.take().unwrap()
    }};
}
pub(crate) use interrupt;

/// Yield an interrupt and get the next resolved value or None
/// expecting the next input to be a ResolvedValue variant
macro_rules! interrupt_with_maybe_value {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        let res = $input.take().unwrap();
        match res {
            crate::runtime::execution::execution_loop::InterruptProvider::ResolvedValue(value) => value,
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

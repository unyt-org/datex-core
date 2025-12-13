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
        // for step in $iterator {
        //     match step {
        //         $(
        //             $pattern => {break $body},
        //         )+
        //         step => yield step,
        //     }
        // }
        
        loop {
            let step = $iterator.next();
            if let Some(step) = step  {
                match step {
                    $(
                        Some($pattern) => break Some($body),
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
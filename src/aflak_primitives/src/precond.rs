/// Try to convert an integer into a usize.
/// Return a `Result<usize, IOErr>`.
macro_rules! try_into_unsigned {
    ($value: ident) => {
        if $value >= 0 {
            Ok($value as usize)
        } else {
            Err($crate::IOErr::UnexpectedInput(format!(
                "'{}' must be positive, but got {}",
                stringify!($value),
                $value
            )))
        }
    };
}

/// Check that a custom condition is fulfilled.
macro_rules! precheck {
    ($start: ident $op :tt $end: ident) => {
        if $start $op $end {
            Ok(())
        } else {
            Err($crate::IOErr::UnexpectedInput(format!(
                "Expected {} {} {}, but got {} {} {}",
                stringify!($start),
                stringify!($op),
                stringify!($end),
                $start,
                stringify!($op),
                $end,
            )))
        }
    };
    ($cond: expr, $($arg:tt)*) => {
        if $cond {
            Ok(())
        } else {
            Err($crate::IOErr::UnexpectedInput(format!($($arg)*)))
        }
    };
}

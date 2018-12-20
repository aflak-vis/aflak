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

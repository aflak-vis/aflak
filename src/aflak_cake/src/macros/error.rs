use super::DSTError;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum MacroEvaluationError<E> {
    DSTError(DSTError<E>),
}

impl<E: fmt::Debug + fmt::Display> error::Error for MacroEvaluationError<E> {
    fn description(&self) -> &'static str {
        "MacroEvaluationError"
    }
}

impl<E: fmt::Display> fmt::Display for MacroEvaluationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let MacroEvaluationError::DSTError(cause) = self;
        write!(f, "Error on evaluation macro. Caused by: {}", cause)
    }
}

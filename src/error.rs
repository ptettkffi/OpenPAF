use std::error::Error;
use std::fmt;

pub struct PafError {
    message: String
}

impl Error for PafError {}

impl fmt::Display for PafError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Debug for PafError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An error occured at file: {}, line: {}. Error message: {}", file!(), line!(), self.message)
    }
}

impl PafError {
    pub fn create_error(message: &str) -> Box<PafError> {
        Box::new(PafError{message: String::from(message)})
    }
}

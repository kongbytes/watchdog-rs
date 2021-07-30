use std::fmt::{Display, Formatter, Result};

#[derive(Debug,Clone)]
pub struct Error {
    pub message: String,
    pub details: Option<String>
}

impl Error {

    pub fn new(message: &'static str, details: impl Display) -> Self {

        Error {
            message: message.to_string(),
            details: Some(details.to_string())
        }
    }

    pub fn basic(message: String) -> Self {

        Error {
            message,
            details: None
        }
    }

}

impl Display for Error {

    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

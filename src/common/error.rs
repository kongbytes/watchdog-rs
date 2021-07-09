use std::fmt::{Display, Formatter, Result};

#[derive(Debug,Clone)]
pub struct ServerError {
    pub message: String,
    pub details: Option<String>
}

impl ServerError {

    pub fn new(message: &'static str, details: impl Display) -> Self {

        ServerError {
            message: message.to_string(),
            details: Some(details.to_string())
        }
    }

    pub fn basic(message: String) -> Self {

        ServerError {
            message,
            details: None
        }
    }

}

impl Display for ServerError {

    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

// ---

#[derive(Debug,Clone)]
pub struct RelayError {
    pub message: String,
    pub details: Option<String>
}

impl RelayError {

    pub fn new(message: &'static str, details: impl Display) -> Self {

        RelayError {
            message: message.to_string(),
            details: Some(details.to_string())
        }
    }

}

impl Display for RelayError {

    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

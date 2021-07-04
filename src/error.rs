use std::fmt::{Display, Formatter, Result};

#[derive(Debug,Clone)]
pub struct ServerError {
    pub message: String
}

impl Display for ServerError {

    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

// ---

#[derive(Debug,Clone)]
pub struct RelayError {
    pub message: String
}

impl Display for RelayError {

    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

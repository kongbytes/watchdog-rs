use std::cmp::PartialEq;
use std::fmt::{Display, Formatter, Result};
use std::process;

use ansi_term::{Colour, Style};

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

    pub fn exit(&self, message: &str, help_message: &str) -> ! {

        let heading = Style::new().bold().fg(Colour::Red);
        let bold = Style::new().bold();
        let heading_msg = heading.paint("✗ Critical error:");

        eprintln!("{} {}", heading_msg, self.message);
        eprintln!("  {} {}", bold.paint("Context:"), message);
        eprintln!("  {} {}", bold.paint("Debug:"), help_message);

        if let Some(details) = &self.details {
            eprintln!("  {}", Colour::Yellow.paint(details));
        }
        
        process::exit(1);
    }

}

impl PartialEq for Error {

    fn eq(&self, other: &Error) -> bool {
        self.message == other.message && self.details == other.details
    }
}

impl Display for Error {

    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

impl From<serde_yaml::Error> for Error {

    fn from(yaml_error: serde_yaml::Error) -> Error {
        
        Error {
            message: yaml_error.to_string(),
            details: None
        }
    }

}

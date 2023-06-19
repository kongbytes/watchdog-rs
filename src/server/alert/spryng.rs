use reqwest::{Client, RequestBuilder};
use serde_json::json;

use super::manager::AlertMedium;

pub struct SpryngAlerter {

    id: String,
    token: String,
    default_encoding: String,
    default_originator: String,
    default_route: String,
    recipients: Vec<String>,

}

impl SpryngAlerter {

    pub fn new<M>(id: M, token: M, recipients: Vec<String>) -> Self where M: Into<String> {

        SpryngAlerter {
            id: id.into(),
            token: token.into(),
            default_encoding: "auto".into(),
            default_originator: "watchdog".into(),
            default_route: "business".into(),
            recipients
        }
    }

}

impl AlertMedium for SpryngAlerter {

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn build_request(&self, message: &str) -> RequestBuilder {

        Client::new()
            .post("https://rest.spryngsms.com/v1/messages'")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "body": message,
                "encoding": self.default_encoding,
                "originator": self.default_originator,
                "recipients": self.recipients,
                "route": self.default_route
            }))
    }

}

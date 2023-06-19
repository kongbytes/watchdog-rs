use reqwest::{Client, RequestBuilder};

use super::manager::AlertMedium;

pub struct TelegramAlerter {

    id: String,
    chat_id: String,
    token: String

}

impl TelegramAlerter {

    pub fn new<M>(id: M, chat_id: M, token: M) -> Self where M: Into<String> {

        TelegramAlerter {
            id: id.into(),
            chat_id: chat_id.into(),
            token: token.into()
        }
    }

}

impl AlertMedium for TelegramAlerter {

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn build_request(&self, message: &str) -> RequestBuilder {
        
        let formatted_message = str::replace(message, "-", "\\-");
    
        let notify_route = format!("https://api.telegram.org/bot{}/sendMessage?chat_id={}&parse_mode=MarkdownV2&text={}", self.token, self.chat_id, formatted_message);
    
        /*if options.disable_notifications {
            notify_route.push_str("&silent=true");
        }*/
    
        let http_client = Client::new();
        http_client.get(notify_route)
    }

}

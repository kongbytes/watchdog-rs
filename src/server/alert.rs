use std::str;

use reqwest::Client;

use crate::common::error::Error;

pub struct TelegramOptions {
    pub disable_notifications: bool
}

pub fn display_warning(message: &str) {
    eprintln!("[ALERTER FAILURE] {}", message);
}

pub async fn alert_telegram(token: &str, chat_id: &str, message: &str, options: TelegramOptions) -> Result<(), Error> {

    let formatted_message = str::replace(message, "-", "\\-");

    let mut notify_route = format!("https://api.telegram.org/bot{}/sendMessage?chat_id={}&parse_mode=MarkdownV2&text={}", token, chat_id, formatted_message);

    if options.disable_notifications {
        notify_route.push_str("&silent=true");
    }

    // TODO Not reacting on failure
    let http_client = Client::new();
    http_client.get(&notify_route)
        .send()
        .await
        .map_err(|err| Error::new("Could not send message to Telegram API", err))?;

    Ok(())
}

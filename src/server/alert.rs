use reqwest::Client;

use crate::common::error::ServerError;

pub struct TelegramOptions {
    pub disable_notifications: bool // TODO
}

pub async fn alert_telegram(token: &str, chat_id: &str, message: &str, options: TelegramOptions) -> Result<(), ServerError> {

    let notify_route = format!("https://api.telegram.org/bot{}/sendMessage?chat_id={}&parse_mode=MarkdownV2&text={}", token, chat_id, message);

    // TODO Not reacting on failure
    let http_client = Client::new();
    http_client.get(&notify_route)
        .send()
        .await
        .map_err(|err| ServerError::new("Could not send message to Telegram API", err))?;

    Ok(())
}

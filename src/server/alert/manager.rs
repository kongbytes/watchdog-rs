use std::{str, collections::HashMap, env};

use reqwest::RequestBuilder;

use crate::{common::error::Error, server::config::AlertConfig};

use super::{telegram::TelegramAlerter, spryng::SpryngAlerter};

pub trait AlertMedium {

    fn get_id(&self) -> String;

    fn build_request(&self, message: &str) -> RequestBuilder;

}

pub struct AlertManager {

    mediums: HashMap<String, Box<dyn AlertMedium + Send + Sync + 'static>>

}

impl AlertManager {

    pub fn try_from_config(config: &[AlertConfig]) -> Result<Self, Error> {

        let mut manager = AlertManager {
            mediums: HashMap::new()
        };

        for alerter in config.iter() {

            if alerter.medium == "telegram" {
    
                let alerter_id = &alerter.name;
    
                let chat_env = alerter.chat_env.clone().ok_or(Error::basic("Expected 'chat_env' configuration with Telegram medium"))?;
                let token_env = alerter.token_env.clone().ok_or(Error::basic("Expected 'token_env' configuration with Telegram medium"))?;
    
                let telegram_chat = env::var(chat_env).map_err(|_| Error::basic("Expected Telegram chat ID as environment variable"))?;
                let telegram_token = env::var(token_env).map_err(|_| Error::basic("Expected Telegram token as environment variable"))?;
    
                let telegram = TelegramAlerter::new(alerter_id, &telegram_chat, &telegram_token);
                manager.add_medium(telegram);
    
                continue;
            }
    
            if alerter.medium == "spryng" {
    
                let alerter_id = &alerter.name;
    
                let recipients_env = alerter.recipients_env.clone().ok_or(Error::basic("Expected 'recipients_env' configuration with Spryng medium"))?;
                let token_env = alerter.token_env.clone().ok_or(Error::basic("Expected 'token_env' configuration with Spryng medium"))?;
    
                let spring_recipients = env::var(recipients_env).map_err(|_| Error::basic("Expected Spryng SMS recipients as environment variable"))?;
                let spryng_token = env::var(token_env).map_err(|_| Error::basic("Expected Spryng token as environment variable"))?;
    
                let formatted_recipients: Vec<String> = spring_recipients.split(',')
                    .map(|recipient| recipient.trim().to_string())
                    .collect();
    
                let spryng = SpryngAlerter::new(alerter_id, &spryng_token, formatted_recipients);
                manager.add_medium(spryng);
    
                continue;
            }
    
            Err(Error::basic(format!("Could not find provider {}", alerter.medium)))?;
        }

        Ok(manager)
        
    }

    pub fn add_medium(&mut self, medium: impl AlertMedium + Send + Sync + 'static) {

        self.mediums.insert(medium.get_id(), Box::new(medium));
    }

    pub async fn trigger_all_test_alerts(&self) -> Result<(), Error> {
        
        for medium_id in self.mediums.keys() {

            println!("Trigger test alert for medium {}", medium_id);
            self.alert(Some(medium_id), "This is a watchdog monitoring test message").await?;
        }

        Ok(())
    }

    pub async fn alert(&self, requested_medium_id: Option<&str>, message: &str) -> Result<(), Error> {

        let medium = match requested_medium_id {
            Some(medium_id) => self.mediums.get(medium_id).ok_or_else(|| Error::basic("Could not find requested medium"))?,
            None => self.mediums.values().next().ok_or_else(|| Error::basic("Could not find default medium"))?,
        };

        // TODO Not reacting on failure
        let request = medium.build_request(message);
        let http_response = request.send()
            .await
            .map_err(|err| {
                let error_message = format!("Could not send message to medium {}", medium.get_id());
                Error::new(error_message, err)
            })?;

        let http_status = &http_response.status();
        if http_status.is_client_error() || http_status.is_server_error() {
            let status_err = Error::basic(format!("Expected HTTP OK, but received {} for medium {}", http_status, medium.get_id()));
            Err(status_err)?;
        }
    
        Ok(())
    }    

}

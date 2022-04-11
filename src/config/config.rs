use crate::config::auth_options::AuthOptions;
use crate::error::RauthyError;
use std::net::SocketAddr;
use tokio::fs::File;
use tokio::prelude::*; // for write_all()

#[derive(Clone, Debug)]
pub struct Config {
    pub listen: SocketAddr,
    pub message: String,
    pub auth_file: Option<String>,
    pub auth_options: AuthOptions,
    pub include_user_header: bool,
    pub ignore_ip: bool,
}

impl Config {
    pub async fn new() -> Result<Self, RauthyError> {
        dotenv::dotenv().ok();
        let listen: SocketAddr = dotenv::var("LISTEN")
            .unwrap_or("127.0.0.1:3031".to_string())
            .parse()
            .unwrap();
        let message = dotenv::var("BASIC_AUTH_MESSAGE").unwrap_or("Rauthy says no!".to_string());
        let auth_file = dotenv::var("AUTH_FILE").ok();
        println!("Auth file {:?}", auth_file);
        let include_user_header = dotenv::var("INCLUDE_USER_HEADER")
            .ok()
            .map(|b| b.parse().unwrap_or(false))
            .unwrap_or_else(|| false);
        let ignore_ip = dotenv::var("IGNORE_IP")
            .ok()
            .map(|b| b.parse().unwrap_or(false))
            .unwrap_or_else(|| false);

        let auth_options = Self::load_file(auth_file.clone()).await.unwrap_or_default();
        Ok(Config {
            listen,
            message,
            auth_file,
            auth_options,
            include_user_header,
            ignore_ip,
        })
    }

    pub async fn load_file(auth_file: Option<String>) -> Result<AuthOptions, RauthyError> {
        if let Some(auth_file) = auth_file {
            let contents = tokio::fs::read_to_string(auth_file.clone()).await?;
            Ok(AuthOptions::from_string(contents))
        } else {
            Ok(AuthOptions::default())
        }
    }

    pub async fn write(&self) -> Result<(), RauthyError> {
        if let Some(auth_file) = self.auth_file.clone() {
            let mut file = File::create(auth_file.clone()).await?;
            let json = serde_json::to_string(&self.auth_options)?;
            file.write_all(json.as_bytes())
                .await
                .map_err(|_| RauthyError::ConfigError(format!("Error writing to {}", auth_file)))?;
            log::debug!("Successfully wrote {}", auth_file)
        } else {
            log::trace!("No config file configured")
        }

        Ok(())
    }
}

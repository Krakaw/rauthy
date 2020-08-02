use crate::config::auth_options::AuthOptions;
use serde_json::Result;
use std::net::{SocketAddr};
use tokio::fs::File;
use tokio::prelude::*; // for write_all()

#[derive(Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub message: String,
    pub auth_file: Option<String>,
    pub auth_options: AuthOptions,
}

impl Config {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();
        let listen: SocketAddr = dotenv::var("LISTEN")
            .unwrap_or("127.0.0.1:3031".to_string())
            .parse()
            .unwrap();
        let message = dotenv::var("BASIC_AUTH_MESSAGE").unwrap_or("".to_string());
        let auth_file = dotenv::var("AUTH_FILE").ok();
        let auth_options = Self::load_file(auth_file.clone()).await;
        Config {
            listen,
            message,
            auth_file,
            auth_options,
        }
    }

    pub async fn load_file(auth_file: Option<String>) -> AuthOptions {
        if let Some(auth_file) = auth_file {
            if let Ok(contents) = tokio::fs::read_to_string(auth_file).await {
                AuthOptions::from_string(contents)
            } else {
                AuthOptions::default()
            }
        } else {
            AuthOptions::default()
        }
    }

    pub async fn write(&self) -> Result<()> {
        if let Some(auth_file) = self.auth_file.clone() {
            if let Ok(mut file) = File::create(auth_file).await {
                let json = serde_json::to_string(&self.auth_options).unwrap();
                file.write_all(json.as_bytes()).await;
            }
        }

        Ok(())
    }
}

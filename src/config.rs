use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;
use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use tokio::fs::File;
use tokio::io::BufReader;
use tokio::prelude::*; // for write_all()

#[derive(Serialize, Deserialize, Default)]
pub struct AuthOptions {
    pub ips: Vec<IpAddr>,
    pub users: HashMap<String, String>,
}

pub struct Config {
    pub listen: SocketAddr,
    pub encoded_auth: String,
    pub message: String,
    pub auth_file: Option<String>,
    pub auth_options: AuthOptions,
}

impl Config {
    pub async fn new() -> Self {
        use dotenv;
        dotenv::dotenv().ok();

        let listen: SocketAddr = dotenv::var("LISTEN")
            .unwrap_or("127.0.0.1:3031".to_string())
            .parse()
            .unwrap();
        let username = dotenv::var("BASIC_AUTH_USER").expect("BASIC_AUTH_USER missing");
        let password = dotenv::var("BASIC_AUTH_PASS").expect("BASIC_AUTH_PASS missing");
        let encoded_auth =
            base64::encode_config(format!("{}:{}", username, password), base64::URL_SAFE);
        let message = dotenv::var("BASIC_AUTH_MESSAGE").unwrap_or("".to_string());
        let auth_file = dotenv::var("AUTH_FILE").ok();
        let auth_options = if let Some(auth_file) = auth_file.clone() {
            if let Ok(contents) = tokio::fs::read_to_string(auth_file).await {
                serde_json::from_str(contents.as_str()).unwrap_or(AuthOptions {
                    ..Default::default()
                })
            } else {
                AuthOptions {
                    ips: vec![],
                    users: HashMap::new(),
                }
            }
        } else {
            AuthOptions {
                ips: vec![],
                users: HashMap::new(),
            }
        };

        Config {
            listen,
            encoded_auth,
            message,
            auth_file,
            auth_options,
        }
    }

    pub async fn write(&self) -> Result<()> {
        if let Some(auth_file) = self.auth_file.clone() {
            eprintln!("auth_file = {:?}", auth_file);
            if let Ok(mut file) = File::create(auth_file).await {
                let json = serde_json::to_string(&self.auth_options).unwrap();
                eprintln!("json = {:?}", json);
                file.write_all(json.as_bytes()).await;
            }
        }

        Ok(())
    }
}

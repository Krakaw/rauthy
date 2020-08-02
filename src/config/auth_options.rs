use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AuthOptions {
    pub ips: Vec<IpAddr>,
    pub users: HashMap<String, String>,
}

impl AuthOptions {
    pub fn from_string(str: String) -> Self {
        serde_json::from_str(str.as_str()).unwrap_or_default()
    }

    pub fn add_user(&mut self, username: String, password: String) {
        let encoded = base64::encode_config(format!("{}:{}", username, password), base64::URL_SAFE);
        self.users.insert(encoded, username);
    }
}

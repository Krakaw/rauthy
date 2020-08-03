use crate::config::command::UserCommand;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;

#[derive(Hash, Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
pub struct Username(String);
impl std::fmt::Display for Username {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<String> for Username {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }

    fn ne(&self, other: &String) -> bool {
        !self.eq(other)
    }
}

impl From<String> for Username {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AuthOptions {
    pub ips: HashMap<IpAddr, Vec<Username>>,
    pub passwords: HashMap<String, Username>,
    pub commands: HashMap<Username, Vec<UserCommand>>,
}

impl AuthOptions {
    pub fn from_string(str: String) -> Self {
        serde_json::from_str(str.as_str()).unwrap_or_default()
    }

    pub fn add_user(&mut self, username: String, password: String) {
        let encoded = base64::encode_config(format!("{}:{}", username, password), base64::URL_SAFE);
        self.passwords.insert(encoded, username.into());
    }

    pub fn remove_user(&mut self, username: String) {
        let empties: Vec<_> = self
            .passwords
            .iter()
            .filter(|(_, v)| v == &&username)
            .map(|(k, _)| k.clone())
            .collect();
        for empty in empties {
            self.passwords.remove(&empty);
        }
    }
}

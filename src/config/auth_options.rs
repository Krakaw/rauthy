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

impl From<&str> for Username {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AuthOptions {
    pub ips: HashMap<IpAddr, Vec<Username>>,
    pub passwords: HashMap<String, Username>,
    pub commands: HashMap<Username, Vec<UserCommand>>,
    pub tokens: HashMap<String, Username>,
}

impl AuthOptions {
    pub fn from_string(str: String) -> Self {
        serde_json::from_str(str.as_str()).unwrap_or_default()
    }

    pub fn add_password(&mut self, username: String, password: String) {
        let encoded = base64::encode_config(format!("{}:{}", username, password), base64::URL_SAFE);
        self.passwords.insert(encoded, username.into());
    }

    pub fn remove_password_by_user(&mut self, username: String) {
        let passwords_for_user: Vec<_> = self
            .passwords
            .iter()
            .filter(|(_, u)| u == &&username)
            .map(|(k, _)| k.clone())
            .collect();
        for password in passwords_for_user {
            self.passwords.remove(&password);
        }
    }

    pub fn check_token(&mut self, token: &String) -> Option<Username> {
        self.tokens.get(token).map(|u| u.clone())
    }

    pub fn add_token(&mut self, token: String, username: Username) {
        self.tokens.insert(token, username);
    }

    pub fn remove_token(&mut self, token: &String) {
        self.tokens.remove(token);
    }

    pub fn clear_tokens(&mut self) {
        self.tokens = HashMap::new();
    }

    pub fn add_command(&mut self, username: &Username, command: UserCommand) {
        let command_name = command.name.clone();
        if command_name.is_some() {
            self.remove_command_by_name(username, command_name.unwrap());
        }
        let commands = self.commands.entry(username.clone()).or_insert(vec![]);
        commands.push(command);
    }

    pub fn remove_command_by_name(&mut self, username: &Username, command_name: String) {
        let commands = self.commands.entry(username.clone()).or_insert(vec![]);
        commands.drain_filter(|c| c.name.contains(&command_name));
    }

    pub fn remove_command_by_index(&mut self, username: &Username, command_index: usize) {
        let commands = self.commands.entry(username.clone()).or_insert(vec![]);
        if commands.len() > command_index {
            commands.remove(command_index);
        }
    }

    pub fn remove_all_commands(&mut self, username: Option<Username>) {
        if username.is_some() {
            self.commands.remove(&username.unwrap());
        } else {
            self.commands.clear();
        }
    }

    pub fn add_ip_and_user(&mut self, ip: IpAddr, username: Option<&Username>) {
        let entry = self.ips.entry(ip).or_insert(vec![]);
        if let Some(username) = username {
            if entry.iter().filter(|u| u == &username).count() == 0 {
                entry.push(username.clone());
            }
        }
    }

    pub fn remove_ip(&mut self, ip: &IpAddr) {
        self.ips.remove(ip);
    }
}

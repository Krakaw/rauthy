use crate::error::NginxAuthError::ConfigError;
use serde::export::Formatter;
use std::error::Error;
use std::fmt::{Display, Result};
use warp::Rejection;

#[derive(Debug)]
pub enum NginxAuthError {
    Generic,
    CommandError(String),
    ServerError(String),
    ConfigError(String),
    UserCommandError(String),
}

impl Error for NginxAuthError {}

impl Display for NginxAuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            NginxAuthError::CommandError(s) => write!(f, "Command Execution Error: {}", s),
            NginxAuthError::ServerError(s) => write!(f, "Server Error: {}", s),
            NginxAuthError::ConfigError(s) => write!(f, "Config Error: {}", s),
            NginxAuthError::Generic => write!(f, "General Error"),
            NginxAuthError::UserCommandError(s) => write!(f, "User Command Error: {}", s),
        }
    }
}

impl From<std::io::Error> for NginxAuthError {
    fn from(e: std::io::Error) -> Self {
        ConfigError(e.to_string())
    }
}

impl From<serde_json::Error> for NginxAuthError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError(e.to_string())
    }
}

impl warp::reject::Reject for NginxAuthError {}

impl From<NginxAuthError> for Rejection {
    fn from(e: NginxAuthError) -> Self {
        warp::reject::custom(e)
    }
}

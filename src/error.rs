use crate::error::RauthyError::ConfigError;
use serde::export::Formatter;
use std::error::Error;
use std::fmt::{Display, Result};
use warp::Rejection;

#[derive(Debug)]
pub enum RauthyError {
    Generic,
    CommandError(String),
    ServerError(String),
    ConfigError(String),
    UserCommandError(String),
    RegexError(String),
}

impl Error for RauthyError {}

impl Display for RauthyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            RauthyError::CommandError(s) => write!(f, "Command Execution Error: {}", s),
            RauthyError::ServerError(s) => write!(f, "Server Error: {}", s),
            RauthyError::ConfigError(s) => write!(f, "Config Error: {}", s),
            RauthyError::Generic => write!(f, "General Error"),
            RauthyError::UserCommandError(s) => write!(f, "User Command Error: {}", s),
            RauthyError::RegexError(s) => write!(f, "Regex Error: {}", s),
        }
    }
}

impl From<std::io::Error> for RauthyError {
    fn from(e: std::io::Error) -> Self {
        ConfigError(e.to_string())
    }
}

impl From<serde_json::Error> for RauthyError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError(e.to_string())
    }
}

impl From<regex::Error> for RauthyError {
    fn from(e: regex::Error) -> Self {
        Self::RegexError(e.to_string())
    }
}
impl warp::reject::Reject for RauthyError {}

impl From<RauthyError> for Rejection {
    fn from(e: RauthyError) -> Self {
        warp::reject::custom(e)
    }
}

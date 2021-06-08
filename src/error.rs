use crate::error::RauthyError::ConfigError;
use std::error::Error;
use std::fmt::{Display, Result, Formatter};
use warp::Rejection;
use std::sync::PoisonError;

#[derive(Debug)]
pub enum RauthyError {
    Generic,
    InvalidUserName,
    ConfigPoison,
    CommandError(String),
    ServerError(String),
    ConfigError(String),
    UserCommandError(String),

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
            RauthyError::InvalidUserName => write!(f, "Invalid user name"),
            RauthyError::ConfigPoison => write!(f, "Config lock poisoned")
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

impl<T> From<std::sync::PoisonError<T>> for RauthyError {
    fn from(_: PoisonError<T>) -> Self {
        RauthyError::Generic
    }
}

impl warp::reject::Reject for RauthyError {

}

// impl From<RauthyError> for Rejection {
//     fn from(e: RauthyError) -> Self {
//         warp::reject::custom(e)
//     }
// }

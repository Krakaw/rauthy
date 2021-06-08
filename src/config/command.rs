use crate::error::RauthyError;
use serde::{Deserialize, Serialize};
use std::process::{Command, Output};
use std::fmt::Formatter;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserCommand {
    pub name: Option<String>,
    pub path: Option<String>,
    pub command: String,
}

impl UserCommand {
    pub fn run(&self) -> Result<Output, RauthyError> {
        let mut command = Command::new(self.command.clone());
        if let Some(current_dir) = self.path.clone() {
            command.current_dir(current_dir);
        }
        Ok(command
            .output()
            .map_err(|e| RauthyError::CommandError(e.to_string()))?)
    }
}

impl std::fmt::Display for UserCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cd {} && {}",
            self.path.as_ref().unwrap_or(&".".to_string()),
            self.command
        )
    }
}

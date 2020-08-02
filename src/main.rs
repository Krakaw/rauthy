mod config;
mod server;

use crate::server::server::{start};
use clap::clap_app;
use config::config::Config;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddUser {
    pub username: String,
    pub password: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = clap_app!(myapp =>
        (version: "0.1.2")
        (author: "Krakaw <41575888+Krakaw@users.noreply.github.com>")
        (about: "An auth resolver for nginx")
        (@subcommand user =>
            (about: "Add basic auth users")
            (@arg username: -u --username +required +takes_value "Sets the username")
            (@arg password: -p --password +required +takes_value "Sets the password")
        )
    )
        .get_matches();

    let mut config = Config::new().await;
    if let Some(matches) = matches.subcommand_matches("user") {
        let user = AddUser {
            username: matches.value_of("username").unwrap().to_string(),
            password: matches.value_of("password").unwrap().to_string(),
        };
        config.auth_options.remove_user(user.username.clone());
        config.auth_options.add_user(user.username, user.password);
        config.write().await;
        return;
    }

    start(config).await;
}

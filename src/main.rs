mod config;
mod server;
use crate::server::server::{start};
use clap::clap_app;
use config::config::Config;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");


#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = clap_app!(nginxaauth =>
        (version: VERSION)
        (name: "nginx-auth")
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
        let username = matches.value_of("username").unwrap().to_string();
        let password = matches.value_of("password").unwrap().to_string();

        config.auth_options.remove_user(username.clone());
        config.auth_options.add_user(username, password);
        config.write().await;
        return;
    }

    start(config).await;
}

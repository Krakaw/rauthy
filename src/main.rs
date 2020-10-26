#![feature(drain_filter)]
#![feature(option_result_contains)]
mod config;
mod error;
mod server;

use crate::config::auth_options::Username;
use crate::config::command::UserCommand;
use crate::error::RauthyError;
use crate::server::server::start;
use clap::{App, Arg, ArgMatches};
use config::config::Config;
use env_logger::Env;
use std::collections::HashMap;
use std::net::IpAddr;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), RauthyError> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    let matches = build_app();

    let mut config = Config::new().await?;
    if let Some(matches) = matches.subcommand_matches("user") {
        let username = matches.value_of("username").unwrap().to_string();
        let password = matches.value_of("password").unwrap().to_string();
        log::info!("Adding user: {}", username);
        config
            .auth_options
            .remove_password_by_user(username.clone());
        config.auth_options.add_password(username.clone(), password);
        config.write().await?;
        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("bypass") {
        if matches.is_present("clear-tokens") {
            log::info!("Clearing tokens");
            config.auth_options.clear_tokens();
        } else if matches.is_present("remove-token") {
            let token = matches
                .value_of("remove-token")
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .unwrap();
            log::info!("Removing token: {:?}", token);
            config.auth_options.remove_token(&token);
        } else if matches.is_present("add-token") {
            let token = matches
                .value_of("add-token")
                .map(|s| s.to_string())
                .unwrap();
            let username = matches.value_of("username").map(|s| s.to_string()).unwrap();
            log::info!("Adding token: {:?}", token);
            config.auth_options.add_token(token, username.into());
        } else {
            log::error!("No parameters supplied");
            return Ok(());
        }

        config.write().await?;

        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("cmd") {
        if matches.is_present("clear") {
            let username = matches.value_of("username").map(|s| s.to_string().into());
            config.auth_options.remove_all_commands(username.clone());
            log::info!(
                "Clearing commands for {}",
                username.unwrap_or("All Users".to_string().into())
            );
            config.write().await?;
            return Ok(());
        }
        let username: Username = matches.value_of("username").unwrap().to_string().into();
        let path = matches.value_of("path").map(|s| s.to_string());
        let command = matches.value_of("command").unwrap().to_string();
        let name = matches.value_of("name").map(|s| s.to_string());

        log::info!(
            "Adding command for user: {} called: {:?} - `cd {:?} && {}`",
            username,
            name,
            path,
            command
        );
        config.auth_options.add_command(
            &username,
            UserCommand {
                name,
                path,
                command,
            },
        );
        config.write().await?;
        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("ip") {
        if matches.is_present("clear") {
            log::info!(
                "Clearing all {} IP addresses",
                config.auth_options.ips.len()
            );
            config.auth_options.ips = HashMap::new();
            config.write().await?;
            return Ok(());
        } else if matches.is_present("add") {
            let ip = matches
                .value_of("add")
                .map(|ip| ip.parse::<IpAddr>().unwrap())
                .unwrap();
            let username: Option<Username> = matches
                .value_of("username")
                .filter(|u| !u.is_empty())
                .map(|u| u.into());

            config.auth_options.add_ip_and_user(ip, username.as_ref());
            config.write().await?;
            log::info!("Adding ip: {} for username: {:?}", ip, username);
            return Ok(());
        } else if matches.is_present("delete") {
            let ip = matches
                .value_of("delete")
                .map(|ip| ip.parse::<IpAddr>().unwrap())
                .unwrap();
            config.auth_options.remove_ip(&ip);
            config.write().await?;
            log::info!("Removed IP address {}", ip);
            return Ok(());
        }
    }

    start(config).await?;
    Ok(())
}

fn build_app() -> ArgMatches {
    App::new("rauthy")
        .version(VERSION)
        .author("Krakaw <41575888+Krakaw@users.noreply.github.com>")
        .about("An auth proxy service")
        .subcommand(
            App::new("user")
                .about("Add basic auth users")
                .arg(
                    Arg::with_name("username")
                        .short('u')
                        .required(true)
                        .takes_value(true)
                        .about("Adds a username for basic auth"),
                )
                .arg(
                    Arg::with_name("password")
                        .short('p')
                        .required(true)
                        .takes_value(true)
                        .about("Adds a password for basic auth"),
                ),
        )
        .subcommand(
            App::new("bypass")
                .about("Manage bypass tokens")
                .arg(
                    Arg::with_name("username")
                        .short('u')
                        .takes_value(true)
                        .required_unless_one(&["remove-token", "clear-tokens"])
                        .about("Username for the token"),
                )
                .arg(
                    Arg::with_name("add-token")
                        .short('a')
                        .takes_value(true)
                        .about("Adds a bypass token available as a query parameter"),
                )
                .arg(
                    Arg::with_name("remove-token")
                        .short('r')
                        .takes_value(true)
                        .about("Removes a bypass token"),
                )
                .arg(
                    Arg::with_name("clear-tokens")
                        .short('c')
                        .about("Clears the bypass tokens"),
                ),
        )
        .subcommand(
            App::new("cmd")
                .about("Add a command for a user")
                .arg(
                    Arg::with_name("username")
                        .short('u')
                        .required_unless("clear")
                        .takes_value(true)
                        .about("Adds a command for this username"),
                )
                .arg(
                    Arg::with_name("name")
                        .short('n')
                        .takes_value(true)
                        .about("A name for the command"),
                )
                .arg(
                    Arg::with_name("command")
                        .short('c')
                        .required_unless("clear")
                        .takes_value(true)
                        .about("The command to be executed"),
                )
                .arg(
                    Arg::with_name("path")
                        .short('p')
                        .required(false)
                        .takes_value(true)
                        .about("The path for command execution"),
                )
                .arg(
                    Arg::with_name("clear").short('C').about(
                        "Clear all commands for this user if supplied otherwise all commands",
                    ),
                ),
        )
        .subcommand(
            App::new("ip")
                .about("Manage ip addresses")
                .arg(
                    Arg::with_name("delete")
                        .short('d')
                        .required_unless_one(&["add", "clear"])
                        .takes_value(true)
                        .about("Delete an authorized IP"),
                )
                .arg(
                    Arg::with_name("add")
                        .short('a')
                        .required_unless_one(&["delete", "clear"])
                        .takes_value(true)
                        .about("Add an authorized IP"),
                )
                .arg(
                    Arg::with_name("username")
                        .short('u')
                        .requires("add")
                        .takes_value(true)
                        .about("Add a username for the IP address"),
                )
                .arg(
                    Arg::with_name("clear")
                        .short('C')
                        .about("Clear all IP addresses"),
                ),
        )
        .get_matches()
}

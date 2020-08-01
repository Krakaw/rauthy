mod config;

use crate::config::Config;
use clap::clap_app;
use log::info;
use serde::Deserialize;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::{HeaderMap, StatusCode};
use warp::Filter;

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
        (@arg server: -s --server "Start the auth server")
        (@subcommand user =>
            (about: "Add basic auth users")
            (@arg username: -u --username +required +takes_value "Sets the username")
            (@arg password: -p --password +required +takes_value "Sets the password")
        )
    )
    .get_matches();

    let config = config::Config::new().await;
    let listen = config.listen.clone();
    let config = Arc::new(Mutex::new(config));
    if let Some(matches) = matches.subcommand_matches("user") {
        let user = AddUser {
            username: matches.value_of("username").unwrap().to_string(),
            password: matches.value_of("password").unwrap().to_string(),
        };
        add_user(user, config.clone()).await;
    }
    let config = warp::any().map(move || Arc::clone(&config));

    if matches.is_present("server") {
        let ips = warp::header::headers_cloned().map(|headers: HeaderMap| {
            if headers.contains_key("http-client-ip") {
                headers
                    .get("http-client-ip")
                    .map(|h| IpAddr::from_str(h.to_str().unwrap()).unwrap())
            } else if headers.contains_key("x-forwarded-for") {
                headers
                    .get("x-forwarded-for")
                    .map(|h| IpAddr::from_str(h.to_str().unwrap()).unwrap())
            } else {
                None
            }
        });
        let routes = warp::path::end()
            .and(warp::get())
            .and(config.clone())
            .and(ips)
            .and(warp::header::optional::<String>("authorization"))
            .and_then(auth)
            .or(warp::post()
                .and(warp::body::json())
                .and(config.clone())
                .and_then(add_user))
            .or(warp::path("status").map(|| StatusCode::OK));

        warp::serve(routes).run(listen).await;
    }
}

async fn add_user(
    body: AddUser,
    config: Arc<Mutex<Config>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut config = config.lock().await;
    let encoded = base64::encode_config(
        format!("{}:{}", body.username, body.password),
        base64::URL_SAFE,
    );
    config.auth_options.users.insert(encoded, body.username);
    config.write().await;

    Ok(StatusCode::CREATED)
}

async fn auth(
    config: Arc<Mutex<Config>>,
    client_ip: Option<IpAddr>,
    auth_header: Option<String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut config = config.lock().await;
    let mut authorized = false;
    if client_ip.is_some() {
        let client_ip = client_ip.unwrap();
        if config
            .auth_options
            .ips
            .iter()
            .find(|ip| **ip == client_ip)
            .is_some()
        {
            authorized = true;
        }

        //Check the auth
        if let Some(auth_header) = auth_header {
            if let Some(user) = config
                .auth_options
                .users
                .get(&auth_header.replace("Basic ", ""))
            {
                info!(
                    "Successful Authentication for '{}' from '{}'",
                    user,
                    client_ip.clone()
                );
                authorized = true;
                config.auth_options.ips.push(client_ip);
                config.write().await;
            }
        }
    }

    let reply = warp::reply::reply();
    let result = if authorized {
        (StatusCode::OK, "X-Pre-Authenticated", "True".to_string())
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "WWW-Authenticate",
            format!("Basic realm=\"{}\"", config.message),
        )
    };
    let reply = warp::reply::with_status(reply, result.0);
    Ok(warp::reply::with_header(reply, result.1, result.2))
}

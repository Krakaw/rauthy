use crate::config::config::Config;
use crate::error::NginxAuthError;
use log::info;
use serde::Deserialize;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::filters::path::Tail;
use warp::http::{HeaderMap, StatusCode};
use warp::Filter;

#[derive(Deserialize)]
pub struct AddUser {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Default)]
struct AuthQuery {
    token: Option<String>,
}

pub async fn start(config: Config) -> Result<(), NginxAuthError> {
    let listen = config.listen.clone();
    let config = Arc::new(Mutex::new(config));
    let config = warp::any().map(move || Arc::clone(&config));

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

    let status_route = warp::path("status").map(|| StatusCode::OK);
    let user_route = warp::path("user")
        .and(warp::post())
        .and(warp::body::json())
        .and(config.clone())
        .and_then(add_user);
    let auth_route = warp::any()
        .and(config.clone())
        .and(ips)
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::header::optional::<String>("x-bypass-token"))
        .and(warp::query().map(|r: AuthQuery| r.token))
        .and(warp::path::tail().map(|s: Tail| s.as_str().to_string()))
        .and_then(auth);
    let routes = user_route.or(status_route).or(auth_route);

    warp::serve(routes).run(listen).await;
    Ok(())
}

pub async fn add_user(
    user: AddUser,
    config: Arc<Mutex<Config>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut config = config.lock().await;
    config.auth_options.remove_user(user.username.clone());
    config.auth_options.add_user(user.username, user.password);
    config.write().await?;

    Ok(StatusCode::CREATED)
}

async fn auth(
    config: Arc<Mutex<Config>>,
    client_ip: Option<IpAddr>,
    auth_header: Option<String>,
    bypass_token_header: Option<String>,
    bypass_token_query: Option<String>,
    bypass_token_path: String,
) -> Result<impl warp::Reply, warp::Rejection> {
    log::debug!(
        "Auth request from {:?} with auth {:?} and query token {:?} header token {:?} path token {:?}",
        client_ip.clone(),
        auth_header.clone(),
        bypass_token_query.clone(),
        bypass_token_header.clone(),
        bypass_token_path.clone()
    );
    let mut config = config.lock().await;

    let mut authorized = {
        let mut bypass_authorized = false;
        // Check the bypass query param
        if bypass_token_query.is_some() {
            bypass_authorized = config
                .auth_options
                .check_token(&bypass_token_query.unwrap())
                .is_some();
        } else if bypass_token_header.is_some() {
            bypass_authorized = config
                .auth_options
                .check_token(&bypass_token_header.unwrap())
                .is_some();
        } else {
            let parts = bypass_token_path.split('/').collect::<Vec<&str>>();
            if !parts.is_empty() {
                let token = parts
                    .last()
                    .map(|s| s.to_string())
                    .unwrap_or("".to_string());
                bypass_authorized = config.auth_options.check_token(&token).is_some()
            }
        }
        bypass_authorized
    };

    if client_ip.is_some() {
        let client_ip = client_ip.unwrap();
        let ip_exists = config.auth_options.ips.contains_key(&client_ip);
        if ip_exists {
            log::debug!("IP found, authorizing");
            authorized = true;
        }

        //Check the auth
        if let Some(auth_header) = auth_header {
            let map = &config.auth_options.passwords.clone();
            let user = map.get(&auth_header.replace("Basic ", ""));
            if user.is_some() {
                log::debug!("Found user {:?}", user);
                let user = user.unwrap();
                authorized = true;
                let entry = config.auth_options.ips.entry(client_ip).or_insert(vec![]);
                if entry.iter().filter(|u| u == &user).count() == 0 {
                    entry.push(user.clone());
                }

                config.write().await?;
                info!(
                    "Successful Authentication for '{}' from '{}' - adding ip to allowlist",
                    user,
                    client_ip.clone()
                );

                if let Some(commands) = config.auth_options.commands.get(user) {
                    for command in commands {
                        log::debug!("Executing command {}", command);
                        let output = command.run();
                        log::trace!("Output results {:#?}", output);
                    }
                };
            }
        }
    }

    let reply = warp::reply::reply();
    let result = if authorized {
        (StatusCode::OK, "X-Pre-Authenticated", "True".to_string())
    } else {
        log::debug!("Invalid credentials or IP, requesting auth.");
        (
            StatusCode::UNAUTHORIZED,
            "WWW-Authenticate",
            format!("Basic realm=\"{}\"", config.message),
        )
    };
    let reply = warp::reply::with_status(reply, result.0);
    Ok(warp::reply::with_header(reply, result.1, result.2))
}

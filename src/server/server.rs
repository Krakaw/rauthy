use crate::config::auth_options::Username;
use crate::config::command::UserCommand;
use crate::config::config::Config;
use crate::error::RauthyError;
use crate::server::server::AuthenticationType::{
    BasicAuth, BypassTokenHeader, BypassTokenPath, BypassTokenQuery, ClientIp, Unauthenticated,
};
use serde::Deserialize;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::filters::path::Tail;
use warp::http::response::Builder;
use warp::http::{HeaderMap, HeaderValue, StatusCode};
use warp::{Filter, Reply};

#[derive(Deserialize)]
pub struct AddUser {
    pub username: String,
    pub password: Option<String>,
    pub token: Option<String>,
    pub command: Option<UserCommand>,
}

#[derive(Deserialize, Default)]
struct AuthQuery {
    token: Option<String>,
}

#[derive(Debug, PartialEq)]
enum AuthenticationType {
    BasicAuth,
    BypassTokenHeader,
    BypassTokenQuery,
    BypassTokenPath,
    ClientIp,
    Unauthenticated,
}

pub async fn start(config: Config) -> Result<(), RauthyError> {
    let listen = config.listen.clone();
    log::info!("Starting Rauthy on: {:?}", listen);
    let config = Arc::new(Mutex::new(config));
    let config = warp::any().map(move || Arc::clone(&config));

    let ips = warp::header::headers_cloned().map(|headers: HeaderMap| {
        if headers.contains_key("http-client-ip") {
            headers
                .get("http-client-ip")
                .map(|h| h.to_str().unwrap_or(""))
                .filter(|s| !s.is_empty())
                .map(|h| IpAddr::from_str(h))
                .filter(|ip| ip.is_ok())
                .map(|ip| ip.unwrap())
        } else if headers.contains_key("x-forwarded-for") {
            headers
                .get("x-forwarded-for")
                .map(|h| h.to_str().unwrap_or(""))
                .filter(|s| !s.is_empty())
                .map(|h| IpAddr::from_str(h))
                .filter(|ip| ip.is_ok())
                .map(|ip| ip.unwrap())
        } else {
            None
        }
    });

    let status_route = warp::path("status").map(|| StatusCode::OK);
    let reload_route = warp::path("reload")
        .and(config.clone())
        .and_then(reload_config);
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
    let routes = user_route.or(reload_route).or(status_route).or(auth_route);

    warp::serve(routes).run(listen).await;
    Ok(())
}

pub async fn reload_config(config: Arc<Mutex<Config>>) -> Result<impl Reply, warp::Rejection> {
    log::info!("Reloading config");
    let new_conf = Config::new().await?;
    let mut config = config.lock().await;
    config.auth_options = new_conf.auth_options;
    Ok(StatusCode::OK)
}

#[derive(Debug)]
struct InvalidUserName;

impl warp::reject::Reject for InvalidUserName {}

pub async fn add_user(
    user: AddUser,
    config: Arc<Mutex<Config>>,
) -> Result<impl Reply, warp::Rejection> {
    let mut config = config.lock().await;
    let username = user.username.trim().to_string();
    if username.is_empty() {
        log::error!("Empty username");
        return Err(warp::reject::custom(InvalidUserName));
    }
    if let Some(password) = user
        .password
        .map(|p| p.to_string())
        .filter(|p| !p.is_empty())
    {
        config
            .auth_options
            .remove_password_by_user(username.clone());
        config.auth_options.add_password(username.clone(), password);
        log::info!("Added Basic auth for user: {}", username);
    }

    if let Some(token_string) = user.token.map(|t| t.to_string()).filter(|t| !t.is_empty()) {
        config.auth_options.remove_token(&token_string);
        config
            .auth_options
            .add_token(token_string, username.clone().into());
        log::info!("Added Bypass token auth for user: {}", username);
    }

    if let Some(command) = user.command {
        config
            .auth_options
            .add_command(&username.clone().into(), command);
    }
    config.write().await?;
    log::info!("Stored user details for: {}", username);
    Ok(StatusCode::CREATED)
}

async fn auth(
    config: Arc<Mutex<Config>>,
    client_ip: Option<IpAddr>,
    auth_header: Option<String>,
    bypass_token_header: Option<String>,
    bypass_token_query: Option<String>,
    bypass_token_path: String,
) -> Result<impl Reply, warp::Rejection> {
    log::debug!(
        "Auth request from {:?} with auth {:?} and query token {:?} header token {:?} path token {:?}",
        client_ip.clone(),
        auth_header.clone(),
        bypass_token_query.clone(),
        bypass_token_header.clone(),
        bypass_token_path.clone()
    );
    let mut config = config.lock().await;
    let mut logged_in_user: Option<Username> = None;
    let mut authorized = Unauthenticated;

    if client_ip.is_some() {
        let client_ip = client_ip.unwrap_or(IpAddr::from([0, 0, 0, 0]));
        let ip_exists = config.auth_options.ips.contains_key(&client_ip);
        if ip_exists {
            log::debug!("IP found, authorizing");
            authorized = ClientIp;
        } else {
            log::debug!("IP not authorized");
        }
    }

    if authorized == Unauthenticated && auth_header.is_some() {
        //Check the basic auth
        let password_map = &config.auth_options.passwords.clone();
        let auth_header = auth_header.unwrap();
        logged_in_user = password_map
            .get(&auth_header.replace("Basic ", ""))
            .cloned();
        if logged_in_user.is_some() {
            log::debug!("Found basic auth user {:?}", logged_in_user);
            authorized = BasicAuth;
        } else {
            log::debug!("No basic auth user found.");
        }
    }

    if authorized == Unauthenticated && bypass_token_query.is_some() {
        if let Some(user) = config
            .auth_options
            .check_token(&bypass_token_query.unwrap())
        {
            authorized = BypassTokenQuery;
            logged_in_user = Some(user.clone());
            log::debug!("Query token matched user: {:?}", logged_in_user);
        };
    }

    if authorized == Unauthenticated && bypass_token_header.is_some() {
        if let Some(user) = config
            .auth_options
            .check_token(&bypass_token_header.unwrap())
        {
            authorized = BypassTokenHeader;
            logged_in_user = Some(user.clone());
            log::debug!("Header token matched user: {:?}", logged_in_user);
        };
    }

    if authorized == Unauthenticated && !bypass_token_path.trim().is_empty() {
        let parts = bypass_token_path.split('/').collect::<Vec<&str>>();
        let token = parts
            .last()
            .map(|s| s.to_string())
            .unwrap_or("".to_string());
        if let Some(user) = config.auth_options.check_token(&token) {
            authorized = BypassTokenPath;
            logged_in_user = Some(user.clone());
            log::debug!("Path token matched user: {:?}", logged_in_user);
        } else {
            log::debug!("No tokens matched");
        };
    }

    if authorized != Unauthenticated && authorized != ClientIp && logged_in_user.is_some() {
        log::debug!("Found user {:?}", logged_in_user);
        let user = logged_in_user.unwrap();
        if let Some(client_ip) = client_ip {
            // Add the client ip
            let entry = config.auth_options.ips.entry(client_ip).or_insert(vec![]);
            if entry.iter().filter(|u| u == &&user).count() == 0 {
                entry.push(user.clone());
            }

            config.write().await?;
            log::info!(
                "Successful Authentication for '{}' from '{}' - adding ip to allow list",
                user,
                client_ip.clone()
            );
        }

        if let Some(commands) = config.auth_options.commands.get(&user) {
            for command in commands {
                log::debug!("Executing command {}", command);
                let output = command.run();
                log::trace!("Output results {:#?}", output);
            }
        };
    }

    let result = match authorized {
        Unauthenticated => {
            log::debug!("Invalid credentials or IP, requesting auth.");
            Builder::new()
                .status(StatusCode::UNAUTHORIZED)
                .header("X-Rauthy-Authenticated", HeaderValue::from_static("False"))
                .header(
                    "WWW-Authenticate",
                    HeaderValue::from_str(format!("Basic realm=\"{}\"", config.message).as_str())
                        .unwrap(),
                )
        }
        _ => {
            let src = format!("{:?}", authorized);
            Builder::new()
                .status(StatusCode::OK)
                .header("X-Rauthy-Authenticated", HeaderValue::from_static("True"))
                .header(
                    "X-Rauthy-Auth-Type",
                    HeaderValue::from_str(src.clone().as_str()).unwrap(),
                )
        }
    };

    Ok(result.body("").unwrap())
}

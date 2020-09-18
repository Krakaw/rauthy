use crate::config::auth_options::Username;
use crate::config::command::UserCommand;
use crate::config::config::Config;
use crate::error::NginxAuthError;
use crate::server::server::AuthenticationType::{
    BasicAuth, BypassTokenHeader, BypassTokenPath, BypassTokenQuery, ClientIp, Unauthenticated,
};
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
    pub password: Option<String>,
    pub token: Option<String>,
    pub command: Option<UserCommand>,
}

#[derive(Deserialize, Default)]
struct AuthQuery {
    token: Option<String>,
}

enum AuthenticationType {
    BasicAuth,
    BypassTokenHeader,
    BypassTokenQuery,
    BypassTokenPath,
    ClientIp,
    Unauthenticated,
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

pub async fn reload_config(
    config: Arc<Mutex<Config>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    log::debug!("Reloading config");
    let new_conf = Config::new().await?;
    let mut config = config.lock().await;
    config.auth_options = new_conf.auth_options;
    Ok(StatusCode::OK)
}

pub async fn add_user(
    user: AddUser,
    config: Arc<Mutex<Config>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut config = config.lock().await;
    let username = user.username;
    let password = user
        .password
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());
    if password.is_some() {
        config
            .auth_options
            .remove_password_by_user(username.clone());
        config
            .auth_options
            .add_password(username.clone(), password.unwrap());
    }

    let token = user
        .token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    if token.is_some() {
        let token_string = token.unwrap();
        config.auth_options.remove_token(&token_string);
        config
            .auth_options
            .add_token(token_string, username.clone().into());
    }

    if user.command.is_some() {}
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
    let mut logged_in_user: Option<&Username> = None;

    let mut authorized = {
        let mut bypass_authorized = Unauthenticated;
        // Check the bypass query param
        if bypass_token_query.is_some() {
            if config
                .auth_options
                .check_token(&bypass_token_query.unwrap())
                .is_some()
            {
                bypass_authorized = BypassTokenQuery;
            };
        } else if bypass_token_header.is_some() {
            if config
                .auth_options
                .check_token(&bypass_token_header.unwrap())
                .is_some()
            {
                bypass_authorized = BypassTokenHeader;
            };
        } else {
            let parts = bypass_token_path.split('/').collect::<Vec<&str>>();
            if !parts.is_empty() {
                let token = parts
                    .last()
                    .map(|s| s.to_string())
                    .unwrap_or("".to_string());
                if config.auth_options.check_token(&token).is_some() {
                    bypass_authorized = BypassTokenPath;
                };
            }
        }

        bypass_authorized
    };

    if client_ip.is_some() {
        let client_ip = client_ip.unwrap();
        let ip_exists = config.auth_options.ips.contains_key(&client_ip);
        if ip_exists {
            log::debug!("IP found, authorizing");
            authorized = ClientIp;
        }
    }

    //Check the basic auth
    let password_map = &config.auth_options.passwords.clone();
    if let Some(auth_header) = auth_header {
        logged_in_user = password_map.get(&auth_header.replace("Basic ", ""));

        if logged_in_user.is_some() {
            log::debug!("Found user {:?}", logged_in_user);
            authorized = BasicAuth;
        }
    }

    if logged_in_user.is_some() {
        log::debug!("Found user {:?}", logged_in_user);
        let user = logged_in_user.unwrap();
        if let Some(client_ip) = client_ip {
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
        }

        if let Some(commands) = config.auth_options.commands.get(user) {
            for command in commands {
                log::debug!("Executing command {}", command);
                let output = command.run();
                log::trace!("Output results {:#?}", output);
            }
        };
    }

    let reply = warp::reply::reply();
    let result = match authorized {
        Unauthenticated => {
            log::debug!("Invalid credentials or IP, requesting auth.");
            (
                StatusCode::UNAUTHORIZED,
                "WWW-Authenticate",
                format!("Basic realm=\"{}\"", config.message),
            )
        }
        _ => (StatusCode::OK, "X-Pre-Authenticated", "True".to_string()),
    };

    let reply = warp::reply::with_status(reply, result.0);
    Ok(warp::reply::with_header(reply, result.1, result.2))
}

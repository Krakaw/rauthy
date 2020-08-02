use crate::config::config::Config;
use crate::AddUser;
use log::info;
use std::error::Error;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::{HeaderMap, StatusCode};
use warp::Filter;

pub async fn start(config: Config) {
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

pub async fn add_user(
    body: AddUser,
    config: Arc<Mutex<Config>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut config = config.lock().await;
    config.auth_options.add_user(body.username, body.password);
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
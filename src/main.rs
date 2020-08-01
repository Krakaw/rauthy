mod config;

use crate::config::Config;
use log::{info, warn};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::{HeaderMap, StatusCode};
use warp::Filter;

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = config::Config::new().await;
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
        .and(config.clone())
        .and(ips)
        .and(warp::header::optional::<String>("authorization"))
        .and_then(auth)
        .or(warp::path("status").map(|| StatusCode::OK));

    warp::serve(routes).run(listen).await;
}

async fn auth(
    config: Arc<Mutex<Config>>,
    client_ip: Option<IpAddr>,
    auth: Option<String>,
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
        if let Some(auth) = auth {
            if auth == format!("Basic {}", config.encoded_auth) {
                info!("Successful Authentication: {}", client_ip.clone());
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

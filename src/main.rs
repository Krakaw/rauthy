use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::{HeaderMap, StatusCode};
use warp::Filter;

pub struct Config {
    pub encoded_auth: String,
    pub message: String,
}

#[tokio::main]
async fn main() {
    use dotenv;
    dotenv::dotenv().ok();

    let listen: SocketAddr = dotenv::var("LISTEN")
        .unwrap_or("127.0.0.1:3031".to_string())
        .parse()
        .unwrap();
    let username = dotenv::var("BASIC_AUTH_USER").expect("BASIC_AUTH_USER missing");
    let password = dotenv::var("BASIC_AUTH_PASS").expect("BASIC_AUTH_PASS missing");
    let encoded_auth =
        base64::encode_config(format!("{}:{}", username, password), base64::URL_SAFE);
    let message = dotenv::var("BASIC_AUTH_MESSAGE").unwrap_or("".to_string());

    let config = Config {
        encoded_auth,
        message,
    };
    let config = Arc::new(config);
    let config = warp::any().map(move || Arc::clone(&config));

    let db = Arc::new(Mutex::new(vec![]));
    let db = warp::any().map(move || Arc::clone(&db));

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
    let routes = warp::path("auth")
        .and(config.clone())
        .and(db.clone())
        .and(ips)
        .and(warp::header::optional::<String>("authorization"))
        .and_then(auth);
    warp::serve(routes).run(listen).await;
}

async fn auth(
    config: Arc<Config>,
    db: Arc<Mutex<Vec<IpAddr>>>,
    client_ip: Option<IpAddr>,
    auth: Option<String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut ips = db.lock().await;

    let mut authorized = false;
    if client_ip.is_some() {
        let client_ip = client_ip.unwrap();
        if ips.iter().find(|ip| **ip == client_ip).is_some() {
            authorized = true;
        }
        //Check the auth
        if let Some(auth) = auth {
            if auth == format!("Basic {}", config.encoded_auth) {
                authorized = true;
                ips.push(client_ip);
            }
        }
    }

    let reply = warp::reply::reply();

    let result = if authorized {
        (StatusCode::OK, "", "".to_string())
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

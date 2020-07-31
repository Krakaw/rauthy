use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::Filter;

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
    let message = dotenv::var("BASIC_AUTH_MESSAGE").unwrap_or("".to_string());
    let encoded_auth=base64::encode_config(format!("{}:{}", username, password), base64::URL_SAFE);

    let config = Arc::new((encoded_auth, message));
    let config = warp::any().map(move || Arc::clone(&config));

    let db = Arc::new(Mutex::new(vec![]));
    let db = warp::any().map(move || Arc::clone(&db));

    let routes = warp::path("auth")
        .and(config.clone())
        .and(db.clone())
        .and(warp::header::optional::<IpAddr>("http-client-ip"))
        .or(warp::header::optional::<IpAddr>("x-forwarded-for"))
        .unify()
        .map(|ip: Option<IpAddr>| ip)
        .and(warp::header::optional::<String>("authorization"))
        .and_then(auth);
    warp::serve(routes).run(listen).await;
}

async fn auth(
    config: Arc<(String, String)>,
    db: Arc<Mutex<Vec<IpAddr>>>,
    client_ip: Option<IpAddr>,
    auth: Option<String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    eprintln!("auth = {:?}", auth);
    eprintln!("config = {:?}", config);
    eprintln!("auth check = {:?}", auth.clone().unwrap() == format!("Basic {}", config.0));
    let mut ips = db.lock().await;

    let mut authorized = false;
    if client_ip.is_some() {
        let client_ip = client_ip.unwrap();
        if ips.iter().find(|ip| **ip == client_ip).is_some() {
            authorized = true;
        }
        //Check the auth
        if let Some(auth) = auth{
            if auth == format!("Basic {}", config.0) {
                authorized = true;
                eprintln!("yay auth = {:?}",authorized);
                ips.push(client_ip);
            }
        }

    }

    let reply = warp::reply::reply();

    eprintln!("authorized = {:?}", authorized);
    let result = if authorized {
        (StatusCode::OK, "", "".to_string())
    } else {
        (StatusCode::UNAUTHORIZED, "WWW-Authenticate", format!("Basic realm=\"{}\"", config.1))
    };
    let reply = warp::reply::with_status(reply, result.0);
    Ok(warp::reply::with_header(reply , result.1, result.2))

}

use std::net::IpAddr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ProxyEndpoint {
    pub forward_url: String,
    pub forward_replace: Option<String>,
    pub client_ip: IpAddr
}
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ProxyOptions {
    pub proxies: Vec<ProxyEndpoint>
}

pub mod client_api;
pub mod core;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use url::Url;

pub fn url_to_socket_addr(url: &Url) -> anyhow::Result<SocketAddr> {
    let ip_addr = match url.host() {
        Some(url::Host::Domain(domain)) if domain == "localhost" => IpAddr::V4(Ipv4Addr::LOCALHOST),
        Some(url::Host::Domain(domain)) => domain.parse()?,
        Some(url::Host::Ipv4(ip)) => IpAddr::V4(ip),
        Some(url::Host::Ipv6(ip)) => IpAddr::V6(ip),
        None => anyhow::bail!("Invalid URL: no host specified"),
    };
    let port = url.port().expect("Invalid URL: no port specified");
    Ok(SocketAddr::new(ip_addr, port))
}

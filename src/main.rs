use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use url::Url;

fn url_to_socket_addr(url: &Url) -> anyhow::Result<SocketAddr> {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let uri = match std::env::var("ROS_MASTER_URI") {
        Ok(v) => Url::parse(v.as_str())?,
        Err(std::env::VarError::NotPresent) => Url::parse("http://0.0.0.0:11311").unwrap(),
        Err(v) => panic!(
            "Unkown error when parsing ROS_MASTER_URI: {}",
            v.to_string()
        ),
    };

    let socket_address = url_to_socket_addr(&uri)?;
    let master = ros_core_rs::core::Master::new(&socket_address);
    master.serve().await
}

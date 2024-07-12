//! This Rust library provides a standalone implementation of the ROS (Robot
//! Operating System) core. It allows you to run a ROS master and communicate with
//! other ROS nodes without relying on an external ROS installation. You can use
//! this library to build ROS nodes entirely in Rust, including publishers and
//! subscribers, without needing to use any other ROS dependencies.
//!
//! # Examples
//! ```
//! use url::Url;
//! async fn demo() -> anyhow::Result<()>{
//!   const ROS_MASTER_URI: &str = "http://0.0.0.0:11311";
//!   let uri = Url::parse(ROS_MASTER_URI).unwrap();
//!   let socket_address = ros_core_rs::url_to_socket_addr(&uri)?;
//!   let master = ros_core_rs::core::Master::new(&socket_address);
//!   master.serve().await
//! }
//! ```
//!
pub mod client_api;
pub mod core;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use url::Url;

mod param_tree;

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

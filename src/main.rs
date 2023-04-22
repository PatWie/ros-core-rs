use url::Url;

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

    let socket_address = ros_core_rs::url_to_socket_addr(&uri)?;
    let master = ros_core_rs::core::Master::new(&socket_address);
    master.serve().await
}

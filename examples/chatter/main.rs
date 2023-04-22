use std::time::Duration;
use url::Url;

use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let t_core = tokio::spawn(async move {
        let uri = Url::parse("http://0.0.0.0:11311").unwrap();
        let socket_address = ros_core_rs::url_to_socket_addr(&uri)?;
        let master = ros_core_rs::core::Master::new(&socket_address);
        master.serve().await
    });

    rosrust::loop_init("talker_listener", 1000);

    let t_talker = tokio::spawn(async move {
        // Create publisher
        let chatter_pub = rosrust::publish("chatter", 100).unwrap();

        let mut count = 0;

        // Create object that maintains 10Hz between sleep requests
        let rate = rosrust::rate(10.0);

        // Breaks when a shutdown signal is sent
        while rosrust::is_ok() {
            // Create string message
            let mut msg = rosrust_msg::std_msgs::String::default();
            msg.data = format!("hello world {}", count);
            log::info!("I wrote {}", msg.data);

            // Send string message to topic via publisher
            chatter_pub.send(msg).unwrap();

            // Sleep to maintain 10Hz rate
            rate.sleep();

            count += 1;
        }
    });
    // Give the publisher some time to publish.
    // TODO(patwie): There must be a way to retry.
    log::info!("Give publisher some time to come up");
    sleep(Duration::from_secs(4)).await;
    log::info!("Create subscriber");

    let t_listener = tokio::spawn(async move {
        // Create subscriber
        // The subscriber is stopped when the returned object is destroyed
        let _subscriber_info =
            rosrust::subscribe("chatter", 2, |v: rosrust_msg::std_msgs::String| {
                // Callback for handling received messages
                log::info!("I heard {}", v.data);
            })
            .unwrap();

        // Block the thread until a shutdown signal is received
        rosrust::spin();
    });

    let (_r1, _r2, _r3) = tokio::join!(t_core, t_talker, t_listener);
    Ok(())
}

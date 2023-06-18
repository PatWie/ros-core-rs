use ros_core_rs::core::MasterClient;
use std::thread;
use tokio::select;
use url::Url;

const ROS_MASTER_URI: &str = "http://0.0.0.0:11311";
const TOPIC_NAME: &str = "/chatter";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Spawn a Tokio task to run the ROS master
    let core_cancel = tokio_util::sync::CancellationToken::new();
    let t_core = tokio::spawn({
        let core_cancel = core_cancel.clone();
        async move {
            let uri = Url::parse(ROS_MASTER_URI).unwrap();
            let socket_address = ros_core_rs::url_to_socket_addr(&uri)?;
            let master = ros_core_rs::core::Master::new(&socket_address);

            select! {
                serve = master.serve() => {
                    serve
                },
                _ = core_cancel.cancelled() => {
                    Ok(())
                }
            }
        }
    });

    // Initialize the ROS node
    rosrust::loop_init("talker_listener", 1000);

    // Spawn a Tokio task to publish messages
    let t_talker = tokio::spawn(async move {
        // Create publisher
        let chatter_pub = rosrust::publish(TOPIC_NAME, 100).unwrap();

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

    // Wait for the publisher to be available
    let master_url = Url::parse(ROS_MASTER_URI).expect("Failed to parse  URL.");
    let master_client = MasterClient::new(&master_url);
    loop {
        let (_, _, published_topics) = master_client.get_published_topics("", "").await.unwrap();
        if published_topics
            .iter()
            .any(|(topic_name, _)| topic_name == TOPIC_NAME)
        {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(1000));
    }

    // Spawn a Tokio task to subscribe to messages
    let t_listener = tokio::spawn(async move {
        // Create subscriber
        // The subscriber is stopped when the returned object is destroyed
        let mut _subscriber_info;
        loop {
            _subscriber_info =
                rosrust::subscribe(TOPIC_NAME, 2, |v: rosrust_msg::std_msgs::String| {
                    // Callback for handling received messages
                    log::info!("I heard {}", v.data);
                });
            if _subscriber_info.is_ok() {
                break;
            }
            log::info!("publisher not found. Will retry until it becomes available...");
            thread::sleep(std::time::Duration::from_millis(1000));
        }
        log::info!("We successfully subscribes to the publisher");

        // Block the thread until a shutdown signal is received
        rosrust::spin();
    });

    tokio::signal::ctrl_c().await?;

    // Wind down clients
    rosrust::shutdown();
    let (_r1, _r2) = tokio::join!(t_talker, t_listener);

    // Wind down core
    core_cancel.cancel();
    let _r3 = tokio::join!(t_core);

    Ok(())
}

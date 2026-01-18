use std::env;

fn main() {
    env_logger::init();

    let master_uri = env::var("ROS_MASTER_URI").unwrap_or_else(|_| "http://localhost:11311".into());
    env::set_var("ROS_MASTER_URI", &master_uri);

    rosrust::init("tf_listener");

    let _sub = rosrust::subscribe("/tf", 10, |msg: rosrust_msg::tf2_msgs::TFMessage| {
        for tf in &msg.transforms {
            println!(
                "{} -> {}: t=[{:.3}, {:.3}, {:.3}] r=[{:.3}, {:.3}, {:.3}, {:.3}]",
                tf.header.frame_id,
                tf.child_frame_id,
                tf.transform.translation.x,
                tf.transform.translation.y,
                tf.transform.translation.z,
                tf.transform.rotation.x,
                tf.transform.rotation.y,
                tf.transform.rotation.z,
                tf.transform.rotation.w,
            );
        }
    })
    .expect("Failed to subscribe to /tf");

    rosrust::spin();
}

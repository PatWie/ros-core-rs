use std::env;

fn main() {
    env_logger::init();

    let master_uri = env::var("ROS_MASTER_URI").unwrap_or_else(|_| "http://localhost:11311".into());
    env::set_var("ROS_MASTER_URI", &master_uri);

    rosrust::init("tf_faker");

    let pub_tf = rosrust::publish("/tf", 10).expect("Failed to create /tf publisher");
    let rate = rosrust::rate(10.0);
    let mut seq = 0u32;

    while rosrust::is_ok() {
        let now = rosrust::now();
        let t = seq as f64 * 0.1;

        let mut msg = rosrust_msg::tf2_msgs::TFMessage::default();
        let mut tf = rosrust_msg::geometry_msgs::TransformStamped::default();

        tf.header.seq = seq;
        tf.header.stamp = now;
        tf.header.frame_id = "world".into();
        tf.child_frame_id = "base_link".into();
        tf.transform.translation.x = t.sin();
        tf.transform.translation.y = t.cos();
        tf.transform.translation.z = 0.0;
        tf.transform.rotation.w = 1.0;

        msg.transforms.push(tf);
        pub_tf.send(msg).ok();

        seq += 1;
        rate.sleep();
    }
}

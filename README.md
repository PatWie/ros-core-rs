# ROS-core implementation in Rust

This provides a stand-alone implementation of the ROS core. It can be used
in combination with https://github.com/adnanademovic/rosrust to get a single
binary that can publish or subscribe to topics.

## Examples

### Standalone ROS core

```bash
# start the ros-core
RUST_LOG=debug cargo run
```

And run any of your ROS stack, eg., the [python chatter example](http://wiki.ros.org/ROS/Tutorials/WritingPublisherSubscriber%28python%29).

### Talker/Listener

This example creates a single binary which contains:
- ROS core
- ROS publisher
- ROS subscriber

```bash
RUST_LOG=info ROSRUST_MSG_PATH=`realpath examples/chatter/msgs` cargo run --example chatter --release
# Stop by Ctrl+C and Ctrl+Z
kill -9 %1
```

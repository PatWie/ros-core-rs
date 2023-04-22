# ROS-core implementation in Rust

![Rust](https://img.shields.io/badge/Rust-1.55+-orange.svg) ![License](https://img.shields.io/badge/license-MIT-blue.svg)

This Rust library provides a standalone implementation of the ROS (Robot
Operating System) core. It allows you to run a ROS master and communicate with
other ROS nodes without relying on an external ROS installation. You can use
this library to build ROS nodes entirely in Rust, including publishers and
subscribers, without needing to use any other ROS dependencies.

## Examples

### Standalone ROS core

To start the ROS core, run the following command:

```bash
# start the ros-core
RUST_LOG=debug cargo run
```

And run any of your ROS stack, eg., the [python chatter example](http://wiki.ros.org/ROS/Tutorials/WritingPublisherSubscriber%28python%29).

### Talker/Listener

This [example](./examples/chatter/main.rs) creates a single binary which contains:

- ROS core
- ROS publisher
- ROS subscriber

To run the talker/listener example, execute the following command:

```bash
RUST_LOG=info ROSRUST_MSG_PATH=`realpath examples/chatter/msgs` cargo run --example chatter --release
# Stop by Ctrl+C and Ctrl+Z
kill -9 %1
```

This example creates a single binary that includes a standalone implementation
of the ROS core, as well as a ROS publisher and ROS subscriber. This
implementation is inspired by the official chatter python example from the ROS
wiki, which demonstrates a simple communication between two nodes using ROS
messages. You can stop the program by using Ctrl+C and Ctrl+Z.

### Debugging with official ROS docker image

To showcase that this ROS core implementation can be used with official ROS
publishers and subscribers in Python, we have provided a debugging script that
launches a ROS Docker image and runs a talker and listener example. To run the
script, execute the following commands:

```bash
# change directory to debugging folder
cd debugging
# make the script executable
chmod +x run.sh
# execute the script
./run.sh
```

This script will download the official ROS Docker image and launch a container
with a ROS environment. Then, it will run a Python script that uses the ROS
talker and listener nodes to communicate with the standalone ROS core
implementation from this repository. This is intended as an example of how to use the
standalone ROS core (ros-core-rs) implementation with other ROS nodes, but it is not
necessary to use this script to use the standalone implementation on its own.

## Contributions

We welcome contributions to this project! If you find a bug or have a feature
request, please create an issue on the GitHub repository. If you want to
contribute code, feel free to submit a pull request.

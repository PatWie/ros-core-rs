#!/bin/bash

# Get the absolute path of the current directory
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

# Run the docker container
sudo docker run \
    --network=host \
    -v "$CURRENT_DIR":/src \
    ros:noetic-perception-buster \
    /usr/bin/python3 /src/talker_listener.py


SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
sudo docker run -v ${SCRIPT_DIR}:/src --network=host ros:noetic-perception-buster /usr/bin/python3 /src/talker_listener.py

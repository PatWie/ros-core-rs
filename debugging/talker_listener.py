#!/usr/bin/env python

import rospy
from std_msgs.msg import String


def talker_listener():
    pub = rospy.Publisher("chatter", String, queue_size=10)
    sub = rospy.Subscriber("chatter", String, callback)

    rospy.init_node("talker_listener", anonymous=True)
    rate = rospy.Rate(10)  # 10 Hz
    while not rospy.is_shutdown():
        hello_str = "hello world %s" % rospy.get_time()
        rospy.loginfo(hello_str)
        pub.publish(hello_str)
        rate.sleep()


def callback(data):
    print(rospy.get_caller_id() + "I heard %s", data.data, flush=True)


if __name__ == "__main__":
    try:
        talker_listener()
    except rospy.ROSInterruptException:
        pass

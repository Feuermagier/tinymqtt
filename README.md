# TinyMQTT
A very tiny no-std MQTT client, mainly for embedded systems. Not very well tested, and only supports a *very* small subset of MQTT - barely enough to publish messages and subscribe to topics.

The client performs no networking itself.
Instead, it writes to and reads from `&[u8]` buffers supplied by the user (and typically received or to be sent via network sockets).

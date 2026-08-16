# MCAP JSON adapter

`quatopsy adapt --format mcap-json` reads an uncompressed MCAP file with exactly one JSON attitude channel.

The channel metadata must declare `frame_from` and `frame_to`. Schema name `quatopsy.adapt.mcap-json/1` or `quatopsy.adapt.ros-json/1`. Each message is `{"t", "x", "y", "z", "w"}` (ROS `xyzw`). Compressed or nested chunks are refused. The adapter never writes a report `result`.

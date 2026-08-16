# SPICE CK type 3 adapter

`quatopsy adapt --format spice-ck` reads a little-endian IEEE (`LTL-IEEE`) DAF/CK file and dumps discrete type 3 pointing samples.

Frames are declared from the kernel's own NAIF integer IDs (`CK-<instrument>` and `NAIF-<frame>`). Encoded SCLK values are emitted as relative seconds without an SCLK kernel; they are not UTC. Angular velocity is not mapped, because SPICE stores it in the base frame. Other CK types, big-endian files, and interpolation are refused. The adapter never writes a report `result`.

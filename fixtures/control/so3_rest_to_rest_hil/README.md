# SO(3) rest-to-rest loopback HIL fixture

`quatopsy control --problem problem.json` sends torque commands across a process boundary to a loopback actuator emulator. Physical actuators are refused.

The command writes `input.csv`, `manifest.json`, and `control.json`. `control.json` never contains a report `result`. This is not hardware qualification and not flight approval.

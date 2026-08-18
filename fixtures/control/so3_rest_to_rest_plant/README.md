# SO(3) rest-to-rest declared plant fixture

`quatopsy control --problem problem.json` runs loopback HIL with declared first-order wheel lag, residual dipole torque, gravity-gradient torque, and gyro angular random walk.

The command writes `input.csv`, `manifest.json`, and `control.json`. `control.json` never contains a report `result`. These models are software. They are not flight hardware and not a qualification record.

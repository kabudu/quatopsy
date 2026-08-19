# SO(3) rest-to-rest declared plant fixture

`quatopsy control --problem problem.json` runs loopback HIL with declared first-order command-to-torque lag, residual dipole torque against a frozen inertial field, gravity-gradient torque against a frozen nadir, wheels, and gyro angular random walk.

The command writes `input.csv`, `manifest.json`, `control.json`, `nav.json`, and `guidance.json`. `control.json`, `nav.json`, and `guidance.json` never contain a report `result`. These models are software. They are not flight hardware, not an orbit, and not a qualification record.

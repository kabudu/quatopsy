# SO(3) rest-to-rest SIL fixture

`quatopsy control --problem problem.json` runs a software-in-the-loop geometric PD controller on a torque-limited rigid body.

The command writes `input.csv`, `manifest.json`, and `control.json`. `control.json` never contains a report `result`. Safety, hard real-time, physical hardware command, and flight approval remain unclaimed.

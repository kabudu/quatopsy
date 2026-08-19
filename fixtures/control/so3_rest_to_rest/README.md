# SO(3) rest-to-rest SIL fixture

`quatopsy control --problem problem.json` runs a software-in-the-loop geometric PD controller on a torque-limited rigid body.

The command writes `input.csv`, `manifest.json`, `control.json`, `nav.json`, and `guidance.json`. `control.json`, `nav.json`, and `guidance.json` never contain a report `result`. Safety, hard real-time, physical hardware command, and flight approval remain unclaimed.

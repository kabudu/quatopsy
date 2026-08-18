# SO(3) rest-to-rest PIL fixture

`quatopsy control --problem problem.json` runs the geometric PD cycle in an isolated child process on the host CPU. The plant stays in the parent process.

The command writes `input.csv`, `manifest.json`, and `control.json`. `control.json` never contains a report `result`. This is not a qualified flight processor, not hard real-time, and not flight approval.

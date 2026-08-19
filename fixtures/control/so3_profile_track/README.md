# SO(3) profile-track SIL fixture

`quatopsy control --problem problem.json` tracks a time-tagged attitude profile with a 6-state MEKF, geometric PD, and a 4-wheel pyramid allocator.

The command writes `input.csv`, `manifest.json`, `control.json`, `nav.json`, and `guidance.json`. None of those JSON documents contains a report `result`. This is software evidence. It is not flight navigation, not hard real-time, and not flight approval.

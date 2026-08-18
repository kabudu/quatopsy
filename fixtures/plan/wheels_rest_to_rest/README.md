# Three-wheel rest-to-rest plan fixture

`quatopsy plan --problem problem.json` generates a rest-to-rest candidate under three orthogonal reaction wheels.

The planner writes `input.csv`, `manifest.json`, and `plan.json`. `plan.json` never contains a report `result`. Safety, global optimality, and actuator command remain unclaimed.

# Spherical rest-to-rest plan fixture

`quatopsy plan --problem problem.json` generates a 90 degree eigenaxis candidate under spherical inertia `1 kg m^2` and a `0.05 N m` torque box.

The planner writes `input.csv` (quaternion and body-rate columns, plus undeclared torque columns for the independent oracle), `manifest.json`, and `plan.json`. `plan.json` never contains a report `result`. Safety, global optimality, and actuator command remain unclaimed.

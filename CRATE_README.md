# Quatopsy

**See where rotations go wrong.**

Quatopsy is early-stage, production-quality research software for local advisory evaluation of quaternion attitude data. It finds, explains, and visualises trajectory defects, produces reviewable evidence, and evaluates bounded controller and trajectory candidates without modifying the recorded input.

```bash
cargo install quatopsy --locked
quatopsy --help
```

The `quatopsy` package installs the command-line tool. The supporting `quatopsy-*` packages expose the schema, analysis, adapter, planning, navigation, guidance, control, and independent-oracle components used by the workspace.

Quatopsy is advisory software. A passing report is not flight approval, actuator permission, certification evidence, or proof of operational safety. Physical hardware, hard real-time execution, and orbit determination remain outside the supported boundary.

See the [repository](https://github.com/kabudu/quatopsy) for the product README, architecture, investigation workflow, contribution guide, security policy, and complete documentation.

# aitios
aitios is a library for simulation of weathering effects and texture synthesis.

# Running tests
Right now, there is no command line interface for aitios and you can only use it by building
a custom simulation with `SimulationBuilder` in code. See integration tests in `tests/*` for
examples on how to set up a simulation.

You can run all tests with:

    cargo test --release

Or only, e.g. multi_weathering_test with:

    cargo test --test multi_weathering_test --release

Iterations of results, including log files, textures, OBJ and MTL files will appear in `test-output/*`.

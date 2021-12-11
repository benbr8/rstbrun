# rstbrun
Command line tool which builds and runs [Rstb](https://github.com/benbr8/rstb) tests using [Icarus Verilog](https://github.com/steveicarus/iverilog).

Other simulators will be added in the future.

rstbrun requires tests to have a `rstb.toml` file in their project directory, which contains locations of HDL sources and the top-level name:
```toml
[test]
toplevel = "top_level_name"

[src]
verilog = [
    "hdl/dut.v",
    "hdl/wrapper.v",
]
```

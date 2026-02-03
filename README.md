# or-tools-rs

Safe Rust bindings for [Google OR-Tools](https://developers.google.com/optimization), with a focus on the **CP-SAT** solver.

This repository contains two crates:

- `or-tools` (this is the crate you typically depend on)
- `or-tools-sys` (low-level build/link + FFI glue)

## What this crate does

`or-tools` provides:

- A safe, ergonomic model builder (`builder::CpModelBuilder`) for building CP-SAT models.
- Generated protobuf types in `proto` (e.g. `CpModelProto`, `CpSolverResponse`, `SatParameters`, ...).
- A thin Rust wrapper over the CP-SAT solver entry points in `ffi`.

Under the hood, the actual solving is performed by the OR-Tools C++ library, linked via `or-tools-sys`.

## Install (git only)

This crate is not on crates.io yet.

Add it as a git dependency in your `Cargo.toml`:

```toml
[dependencies]
or-tools = { git = "https://github.com/romnn/or-tools-rs" }
```

In Rust code, you import it as `or_tools` (hyphens become underscores):

```rust
use or_tools::builder::CpModelBuilder;
```

### Selecting how OR-Tools is obtained

By default, `or-tools` enables the `vendor-prebuilt` backend via `or-tools-sys`.

You can choose a backend using features:

```toml
[dependencies]
or-tools = { git = "https://github.com/romnn/or-tools-rs", default-features = false, features = ["system"] }
```

Available backends/features:

- `vendor-prebuilt` (default)
- `system`
- `build-from-source`
- `static` (only meaningful with `build-from-source`)

See `crates/or-tools-sys/README.md` for environment variables and backend selection details.

## How to use

A minimal CP-SAT example:

```rust
use or_tools::builder::CpModelBuilder;
use or_tools::proto::CpSolverStatus;

fn main() {
    let mut model = CpModelBuilder::default();

    let x = model.new_int_var_with_name([(0, 2)], "x");
    let y = model.new_int_var_with_name([(0, 2)], "y");
    let z = model.new_int_var_with_name([(0, 2)], "z");

    model.add_ne(x, y);

    let response = model.solve();
    println!(
        "{}",
        or_tools::ffi::cp_solver_response_stats(&response, false)
    );

    if response.status() == CpSolverStatus::Optimal {
        println!("x = {}", x.solution_value(&response));
        println!("y = {}", y.solution_value(&response));
        println!("z = {}", z.solution_value(&response));
    }
}
```

Notes:

- This crate exposes the raw protobuf types via `or_tools::proto` if you prefer to build `CpModelProto` directly.
- For solver parameters, use `or_tools::ffi::solve_with_parameters(model, params)` (or build via `builder` then pass `builder.proto()`).

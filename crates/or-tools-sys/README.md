# or-tools-sys

Low-level build/link crate for Google OR-Tools.

This crate is responsible for:

- Compiling the small C++ shim used by the safe Rust API.
- Linking against OR-Tools and its dependencies (e.g. protobuf).

## Backends

`or-tools-sys` supports multiple ways to obtain OR-Tools.

### System (default)

Uses an existing OR-Tools installation.

- Provide `ORTOOLS_PREFIX` (or `ORTOOL_PREFIX`) pointing at an install prefix containing:
  - `include/ortools/...`
  - `lib/` (or `lib64/`) with `libortools` and `libprotobuf`

### Vendored prebuilt

Downloads a prebuilt OR-Tools C++ release tarball during the build.

- Enable: `--features vendor-prebuilt`
- Optional environment overrides:
  - `OR_TOOLS_SYS_PREBUILT_VERSION` (default: `9.15`)
  - `OR_TOOLS_SYS_PREBUILT_BUILD` (default: `6755`)

### Build from source

Builds OR-Tools from source using CMake.

- Enable: `--features build-from-source`
- Source tree location:
  - By default: `<workspace>/vendor/or-tools`
  - Override with `OR_TOOLS_SYS_SOURCE_DIR=/abs/path/to/or-tools`

#### Static linking

To request static linking, enable `--features static`.

Note: static linking is only effective with the `build-from-source` backend.

## Selecting a backend

If multiple backend features are enabled (e.g. via `--all-features`), the build defaults to `system`.

To force a backend, set:

- `OR_TOOLS_SYS_BACKEND=system`
- `OR_TOOLS_SYS_BACKEND=vendor-prebuilt`
- `OR_TOOLS_SYS_BACKEND=build-from-source`

## Build dependencies (build-from-source)

You need the usual C++ toolchain and build utilities.

Minimum requirements (per OR-Tools docs):

- CMake >= 3.24
- A C++20 compiler (GCC 10+ or Clang 12+ typically works)

Typical Ubuntu packages:

- `cmake`
- `ninja-build` (optional)
- `gcc`/`g++` or `clang`
- `make`
- `pkg-config`
- `python3` (often required by OR-Tools build tooling)

If the OR-Tools build fails, the error output usually indicates the missing system dependency.

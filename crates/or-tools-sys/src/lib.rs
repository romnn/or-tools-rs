#![deny(missing_docs)]

//! Low-level build/link crate for Google OR-Tools.

/// Marker constant used to force linking this crate.
///
/// This crate primarily exists for its `build.rs` side effects (compiling the C++
/// shim and linking OR-Tools). Downstream crates should reference this constant
/// to ensure the linker includes this crate.
pub const LINKED: () = ();

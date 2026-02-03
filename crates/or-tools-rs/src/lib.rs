//! The `or_tools` crate provides an interface to [Google CP
//! SAT](https://developers.google.com/optimization/cp/cp_solver).
//!
//! # OR-Tools installation
//!
//! For `or_tools` to work, you need to have a working OR-Tools
//! installation. By default, this crate will use the default C++
//! compiler, and add `/opt/ortools` in the search path. If you want
//! to provide your OR-Tools installation directory, you can define
//! the `ORTOOL_PREFIX` environment variable.
//!
//! # Brief overview
//!
//! The [`builder::CpModelBuilder`] provides an easy interface to
//! construct your problem. You can then solve and access to the
//! solver response easily. Here you can find the translation of the
//! first tutorial in the official documentation of CP SAT:
//!
//! ```
//! # #![allow(clippy::needless_doctest_main)]
//! use or_tools::builder::CpModelBuilder;
//! use or_tools::proto::CpSolverStatus;
//!
//! fn main() {
//!     let mut model = CpModelBuilder::default();
//!
//!     let x = model.new_int_var_with_name([(0, 2)], "x");
//!     let y = model.new_int_var_with_name([(0, 2)], "y");
//!     let z = model.new_int_var_with_name([(0, 2)], "z");
//!
//!     model.add_ne(x, y);
//!
//!     let response = model.solve();
//!     println!(
//!         "{}",
//!         or_tools::ffi::cp_solver_response_stats(&response, false)
//!     );
//!
//!     if response.status() == CpSolverStatus::Optimal {
//!         println!("x = {}", x.solution_value(&response));
//!         println!("y = {}", y.solution_value(&response));
//!         println!("z = {}", z.solution_value(&response));
//!     }
//! }
//! ```

#![allow(unsafe_code)]
#![deny(missing_docs)]

#[allow(dead_code)]
const _LINK_OR_TOOLS_SYS: () = or_tools_sys::LINKED;

/// Model builder for ergonomic and efficient model creation.
pub mod builder;

/// Export of the CP SAT protobufs
#[allow(
    warnings,
    missing_docs,
    clippy::all,
    clippy::pendantic,
    rustdoc::broken_intra_doc_links,
    rustdoc::bare_urls,
    clippy::doc_markdown,
    clippy::doc_overindented_list_items,
    clippy::must_use_candidate,
)]
#[rustfmt::skip]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/operations_research.sat.rs"));
}

/// Interface with the CP SAT functions.
pub mod ffi;

pub use prost;

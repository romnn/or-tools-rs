use crate::proto;
use libc::c_char;
use prost::Message;
use std::ffi::CStr;

unsafe extern "C" {
    fn cp_sat_wrapper_solve(
        model_buf: *const u8,
        model_size: usize,
        out_size: &mut usize,
    ) -> *mut u8;
    fn cp_sat_wrapper_solve_with_parameters(
        model_buf: *const u8,
        model_size: usize,
        params_buf: *const u8,
        params_size: usize,
        out_size: &mut usize,
    ) -> *mut u8;
    fn cp_sat_wrapper_cp_model_stats(model_buf: *const u8, model_size: usize) -> *mut c_char;
    fn cp_sat_wrapper_cp_solver_response_stats(
        response_buf: *const u8,
        response_size: usize,
        has_objective: bool,
    ) -> *mut c_char;
    fn cp_sat_wrapper_validate_cp_model(model_buf: *const u8, model_size: usize) -> *mut c_char;
    fn cp_sat_wrapper_solution_is_feasible(
        model_buf: *const u8,
        model_size: usize,
        solution_buf: *const i64,
        solution_size: usize,
    ) -> bool;
}

/// Solves the given [`CpModelProto`][crate::proto::CpModelProto] and
/// returns an instance of
/// [`CpSolverResponse`][crate::proto::CpSolverResponse].
///
/// # Panics
/// Panics if the model cannot be encoded, if the FFI layer returns a null
/// pointer, or if the solver response cannot be decoded.
#[must_use]
pub fn solve(model: &proto::CpModelProto) -> proto::CpSolverResponse {
    let buf = model.encode_to_vec();
    let mut out_size = 0;
    let res = unsafe { cp_sat_wrapper_solve(buf.as_ptr(), buf.len(), &mut out_size) };
    if res.is_null() {
        std::process::abort();
    }
    let out_slice = unsafe { std::slice::from_raw_parts(res, out_size) };
    let response =
        proto::CpSolverResponse::decode(out_slice).unwrap_or_else(|_| std::process::abort());
    unsafe { libc::free(res.cast()) };
    response
}

/// Solves the given [`CpModelProto`][crate::proto::CpModelProto] with
/// the given parameters.
///
/// # Panics
/// Panics if the model/parameters cannot be encoded, if the FFI layer returns a
/// null pointer, or if the solver response cannot be decoded.
#[must_use]
pub fn solve_with_parameters(
    model: &proto::CpModelProto,
    params: &proto::SatParameters,
) -> proto::CpSolverResponse {
    let model_buf = model.encode_to_vec();
    let params_buf = params.encode_to_vec();

    let mut out_size = 0;
    let res = unsafe {
        cp_sat_wrapper_solve_with_parameters(
            model_buf.as_ptr(),
            model_buf.len(),
            params_buf.as_ptr(),
            params_buf.len(),
            &mut out_size,
        )
    };
    if res.is_null() {
        std::process::abort();
    }
    let out_slice = unsafe { std::slice::from_raw_parts(res, out_size) };
    let response =
        proto::CpSolverResponse::decode(out_slice).unwrap_or_else(|_| std::process::abort());
    unsafe { libc::free(res.cast()) };
    response
}

/// Returns a string with some statistics on the given
/// [`CpModelProto`][crate::proto::CpModelProto].
///
/// # Panics
/// Panics if the model cannot be encoded, if the FFI layer returns a null
/// pointer, or if the returned C string is not valid UTF-8.
#[must_use]
pub fn cp_model_stats(model: &proto::CpModelProto) -> String {
    let model_buf = model.encode_to_vec();
    let char_ptr = unsafe { cp_sat_wrapper_cp_model_stats(model_buf.as_ptr(), model_buf.len()) };
    if char_ptr.is_null() {
        std::process::abort();
    }
    let res = unsafe { CStr::from_ptr(char_ptr) }
        .to_str()
        .unwrap_or_else(|_| std::process::abort())
        .to_owned();
    unsafe { libc::free(char_ptr.cast()) };
    res
}

/// Returns a string with some statistics on the solver response.
///
/// If the second argument is false, we will just display NA for the
/// objective value instead of zero. It is not really needed but it
/// makes things a bit clearer to see that there is no objective.
///
/// # Panics
/// Panics if the response cannot be encoded, if the FFI layer returns a null
/// pointer, or if the returned C string is not valid UTF-8.
#[must_use]
pub fn cp_solver_response_stats(response: &proto::CpSolverResponse, has_objective: bool) -> String {
    let response_buf = response.encode_to_vec();
    let char_ptr = unsafe {
        cp_sat_wrapper_cp_solver_response_stats(
            response_buf.as_ptr(),
            response_buf.len(),
            has_objective,
        )
    };
    if char_ptr.is_null() {
        std::process::abort();
    }
    let res = unsafe { CStr::from_ptr(char_ptr) }
        .to_str()
        .unwrap_or_else(|_| std::process::abort())
        .to_owned();
    unsafe { libc::free(char_ptr.cast()) };
    res
}

/// Verifies that the given model satisfies all the properties
/// described in the proto comments. Returns an empty string if it is
/// the case, otherwise fails at the first error and returns a
/// human-readable description of the issue.
///
/// # Panics
/// Panics if the model cannot be encoded, if the FFI layer returns a null
/// pointer, or if the returned C string is not valid UTF-8.
#[must_use]
pub fn validate_cp_model(model: &proto::CpModelProto) -> String {
    let model_buf = model.encode_to_vec();
    let char_ptr = unsafe { cp_sat_wrapper_validate_cp_model(model_buf.as_ptr(), model_buf.len()) };
    if char_ptr.is_null() {
        std::process::abort();
    }
    let res = unsafe { CStr::from_ptr(char_ptr) }
        .to_str()
        .unwrap_or_else(|_| std::process::abort())
        .to_owned();
    unsafe { libc::free(char_ptr.cast()) };
    res
}

/// Verifies that the given variable assignment is a feasible solution
/// of the given model. The values vector should be in one to one
/// correspondence with the `model.variables()` list of variables.
///
/// # Example
///
/// ```
/// # use cp_sat::builder::CpModelBuilder;
/// # use cp_sat::proto::CpSolverStatus;
/// # use cp_sat::ffi::solution_is_feasible;
/// let mut model = CpModelBuilder::default();
/// let x = model.new_bool_var();
/// let y = model.new_bool_var();
/// model.add_and([x, y]);
/// assert!(solution_is_feasible(model.proto(), &[1, 1]));
/// assert!(!solution_is_feasible(model.proto(), &[1, 0]));
/// assert!(!solution_is_feasible(model.proto(), &[0, 1]));
/// assert!(!solution_is_feasible(model.proto(), &[0, 0]));
/// ```
///
/// # Panics
/// Panics if the model cannot be encoded.
#[must_use]
pub fn solution_is_feasible(model: &proto::CpModelProto, solution: &[i64]) -> bool {
    let model_buf = model.encode_to_vec();
    unsafe {
        cp_sat_wrapper_solution_is_feasible(
            model_buf.as_ptr(),
            model_buf.len(),
            solution.as_ptr(),
            solution.len(),
        )
    }
}

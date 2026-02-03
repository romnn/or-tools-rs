use or_tools::builder::CpModelBuilder;
use or_tools::proto::{CpSolverStatus, SatParameters};

#[test]
fn simple_sat_parameters() {
    let mut model = CpModelBuilder::default();
    let x = model.new_bool_var();
    let params = SatParameters::default();
    let response = model.solve_with_parameters(&params);
    assert_eq!(response.status(), CpSolverStatus::Optimal);
    let _x_value = x.solution_value(&response);
}

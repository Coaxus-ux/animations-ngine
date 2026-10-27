#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_expr::{CompileErrorKind, Env, EvalError, MAX_NESTING, Var, compile};

fn env() -> Env {
    Env {
        time: 0.5,
        frame: 6.0,
        fps: 12.0,
        duration: 2.0,
    }
}

fn eval(source: &str) -> Result<f64, EvalError> {
    compile(source).unwrap().eval(&env())
}

fn kind(source: &str) -> CompileErrorKind {
    compile(source).unwrap_err().kind
}

#[test]
fn arithmetic_follows_precedence() {
    assert_eq!(eval("1 + 2 * 3"), Ok(7.0));
    assert_eq!(eval("(1 + 2) * 3"), Ok(9.0));
    assert_eq!(eval("-2 * -3"), Ok(6.0));
    assert_eq!(eval("7 % 4 - 1"), Ok(2.0));
    assert_eq!(eval("10 / 4"), Ok(2.5));
}

#[test]
fn variables_constants_and_functions_resolve() {
    assert_eq!(eval("f + fps + dur"), Ok(20.0));
    assert_eq!(eval("floor(t * 3)"), Ok(1.0));
    assert_eq!(eval("max(min(1, 2), abs(-3))"), Ok(3.0));
    let half_turn = eval("sin(t * pi)").unwrap();
    assert!((half_turn - 1.0).abs() < 1e-12);
    assert!((eval("cos(tau)").unwrap() - 1.0).abs() < 1e-12);
}

#[test]
fn constant_and_variable_usage_are_reported() {
    assert_eq!(compile("5").unwrap().constant(), Some(5.0));
    let program = compile("t * 2").unwrap();
    assert_eq!(program.constant(), None);
    assert!(program.uses(Var::Time));
    assert!(!program.uses(Var::Frame));
}

#[test]
fn division_by_zero_is_a_non_finite_error() {
    assert_eq!(eval("1 / 0"), Err(EvalError::NonFinite));
}

#[test]
fn compile_errors_carry_kind_and_position() {
    assert_eq!(kind(""), CompileErrorKind::Empty);
    assert_eq!(kind("1 + $"), CompileErrorKind::UnexpectedChar);
    assert_eq!(kind("1 +"), CompileErrorKind::UnexpectedToken);
    assert_eq!(kind("(1"), CompileErrorKind::UnexpectedToken);
    assert_eq!(kind("sin"), CompileErrorKind::MissingCall);
    assert_eq!(kind("t(1)"), CompileErrorKind::NotAFunction);
    assert_eq!(
        kind("min(1)"),
        CompileErrorKind::WrongArity {
            expected: 2,
            found: 1
        }
    );
    let error = compile("t-f").unwrap_err();
    assert_eq!(
        (error.kind, error.start, error.end),
        (CompileErrorKind::UnknownName, 0, 3)
    );
    assert_eq!(compile("t - f").map(|_| ()), Ok(()));
}

#[test]
fn nesting_is_bounded() {
    let ok = format!("{}1{}", "(".repeat(MAX_NESTING), ")".repeat(MAX_NESTING));
    assert_eq!(eval(&ok), Ok(1.0));
    let deep = format!(
        "{}1{}",
        "(".repeat(MAX_NESTING + 1),
        ")".repeat(MAX_NESTING + 1)
    );
    assert_eq!(kind(&deep), CompileErrorKind::TooDeep);
}

#[test]
fn stack_depth_is_bounded() {
    // Right-nested additions keep one value per level on the stack.
    let source = (0..40).map(|_| "1 + (").collect::<String>() + "1" + &")".repeat(40);
    assert_eq!(kind(&source), CompileErrorKind::TooDeep);
    let wide = (0..30).map(|_| "(1+").collect::<String>() + "1" + &")".repeat(30);
    assert_eq!(eval(&wide), Ok(31.0));
}

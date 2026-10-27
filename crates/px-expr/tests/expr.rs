#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_expr::{CompileErrorKind, Ease, Env, EvalError, MAX_NESTING, Var, compile};

fn env() -> Env {
    Env {
        time: 0.5,
        frame: 6.0,
        fps: 12.0,
        duration: 2.0,
        ..Env::default()
    }
}

fn eval(source: &str) -> Result<f64, EvalError> {
    eval_at(source, &env())
}

fn eval_at(source: &str, env: &Env) -> Result<f64, EvalError> {
    compile(source).unwrap().eval(env)
}

fn approx(source: &str, expected: f64) {
    let value = eval(source).unwrap();
    assert!(
        (value - expected).abs() < 1e-9,
        "{source} = {value}, expected {expected}"
    );
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
    assert_eq!(eval("2 ** 3 ** 2"), Ok(512.0));
    assert_eq!(eval("-2 ** 2"), Ok(4.0));
    assert_eq!(eval("2 * 3 ** 2"), Ok(18.0));
}

#[test]
fn comparisons_logic_and_ternary() {
    assert_eq!(eval("1 < 2 && 2 <= 2 && 3 > 2 && 3 >= 3"), Ok(1.0));
    assert_eq!(eval("1 == 2 || 1 != 1"), Ok(0.0));
    assert_eq!(eval("!0 + !5"), Ok(1.0));
    assert_eq!(eval("t > 0.25 ? 10 : 20"), Ok(10.0));
    assert_eq!(eval("0 ? 1 : 0 ? 2 : 3"), Ok(3.0));
    assert_eq!(eval("true ? 1 / 0 * 0 + 4 : 1"), Err(EvalError::NonFinite));
    assert_eq!(eval("false ? 1 / 0 : 4"), Ok(4.0));
}

#[test]
fn literals_with_units() {
    assert_eq!(eval("2s + 500ms"), Ok(2.5));
    assert_eq!(eval("36f"), Ok(3.0));
    assert_eq!(eval("45deg"), Ok(45.0));
    assert_eq!(eval("between('0.25s', 1s)"), Ok(1.0));
    assert_eq!(kind("3px"), CompileErrorKind::InvalidUnit);
    assert_eq!(kind("'pine'"), CompileErrorKind::StringNotAllowed);
    assert_eq!(kind("'open"), CompileErrorKind::UnterminatedString);
}

#[test]
fn variables_constants_and_math_functions() {
    assert_eq!(eval("f + fps + dur + n + i"), Ok(21.0));
    assert_eq!(
        eval("floor(t * 3) + ceil(0.2) + round(-2.5) + sign(-3)"),
        Ok(-2.0)
    );
    assert_eq!(eval("max(min(1, 2, 0), abs(-3), 2)"), Ok(3.0));
    assert_eq!(
        eval("clamp(5, 0, 2) + lerp(0, 10, 0.5) + map(5, 0, 10, 0, 100)"),
        Ok(57.0)
    );
    assert_eq!(
        eval("step(1, 0.5) + step(1, 1) + mod(-1, 4) + snap(7, 4) + snap(2.4)"),
        Ok(14.0)
    );
    approx(
        "smoothstep(0, 2, 1) + fract(2.25) + deg(pi) + rad(180)",
        0.75 + 180.0 + std::f64::consts::PI,
    );
    approx(
        "sin(t * pi) + cos(tau) + tan(0) + atan2(1, 1) * 4",
        2.0 + std::f64::consts::PI,
    );
    approx(
        "sqrt(16) + exp(0) + log(1) + pow(2, 10) + dist(0, 0, 3, 4) + len(6, 8)",
        1044.0 - 1019.0 + 1019.0,
    );
}

#[test]
fn periodic_shapes_and_time_helpers() {
    assert_eq!(
        eval("wave(0) + wave(0.5) + tri(0.25) + saw(1.75)"),
        Ok(2.25)
    );
    assert_eq!(
        eval("square(0.2) + square(0.7) + square(0.7, 0.8)"),
        Ok(2.0)
    );
    approx("cycle(2)", 0.25);
    approx("osc(2) + osc(2, 0, 10)", 1.0 + 10.0);
    assert_eq!(eval("pulse(1, 0.6) + pulse(1)"), Ok(1.0));
}

#[test]
fn time_scopes_shift_the_clock() {
    let late = Env {
        time: 2.5,
        frame: 30.0,
        index: 2.0,
        ..env()
    };
    assert_eq!(eval_at("loop(t, 1s)", &late), Ok(0.5));
    assert_eq!(eval_at("loop(f, 1s)", &late), Ok(6.0));
    assert_eq!(eval_at("delay(t, 1s)", &late), Ok(1.5));
    assert_eq!(eval_at("stagger(t, 0.5)", &late), Ok(1.5));
    let odd = Env {
        frame: 7.0,
        time: 7.0 / 12.0,
        ..env()
    };
    assert_eq!(eval_at("on2(f)", &odd), Ok(6.0));
    assert_eq!(eval_at("on3(f) + hold(f, 4)", &odd), Ok(10.0));
    assert_eq!(
        eval_at("t + loop(t + delay(t, 0.5), 2) + t", &late),
        Ok(2.5 + 0.5 + 2.5)
    );
}

#[test]
fn easing_curves() {
    assert_eq!(eval("ease('linear', 0.25)"), Ok(0.25));
    assert_eq!(eval("ease('step', 0.9) + ease('step', 1)"), Ok(1.0));
    assert_eq!(
        eval("ease('ease-in', 0.5) + ease('ease-out', 0.5)"),
        Ok(1.0)
    );
    assert_eq!(Ease::Smooth.apply(0.5), 0.5);
    for ease in ["smooth", "ease-in-out", "bounce", "elastic"] {
        assert_eq!(eval(&format!("ease('{ease}', 0)")), Ok(0.0), "{ease}");
        assert_eq!(eval(&format!("ease('{ease}', 1)")), Ok(1.0), "{ease}");
    }
    assert_eq!(kind("ease('wobble', 1)"), CompileErrorKind::UnknownEase);
}

#[test]
fn randomness_is_stable_per_seed_and_key() {
    let seeded = |seed: u64, source: &str| eval_at(source, &Env { seed, ..env() }).unwrap();
    assert_eq!(seeded(7, "rand()"), seeded(7, "rand(0)"));
    assert_ne!(seeded(7, "rand(1)"), seeded(7, "rand(2)"));
    assert_ne!(seeded(7, "rand(1)"), seeded(8, "rand(1)"));
    for key in 0..50 {
        let r = seeded(3, &format!("rand({key})"));
        let i = seeded(3, &format!("randi(2, 4, {key})"));
        let v = seeded(3, &format!("range(10, 20, {key})"));
        let p = seeded(3, &format!("pick({key}, 5, 6, 7)"));
        assert!(
            (0.0..1.0).contains(&r) && [2.0, 3.0, 4.0].contains(&i) && (10.0..=20.0).contains(&v)
        );
        assert!([5.0, 6.0, 7.0].contains(&p));
    }
    assert_eq!(seeded(1, "chance(1, 4) + chance(0, 4)"), 1.0);
}

#[test]
fn noise_is_smooth_bounded_and_seeded() {
    for step in 0..40 {
        let x = f64::from(step) * 0.37;
        for source in [
            format!("noise({x})"),
            format!("noise({x}, 1.5)"),
            format!("noise({x}, 1, 2)"),
            format!("fbm({x}, 2)"),
            format!("cells({x}, 0.5)"),
        ] {
            let value = eval(&source).unwrap();
            assert!((0.0..=1.0).contains(&value), "{source} = {value}");
        }
    }
    let near = (eval("noise(1.0)").unwrap() - eval("noise(1.001)").unwrap()).abs();
    assert!(near < 0.01, "noise must be continuous, jumped {near}");
}

#[test]
fn wiggle_loops_seamlessly_in_looping_scenes() {
    let looping = |time: f64| Env {
        time,
        loop_period: Some(2.0),
        ..env()
    };
    let start = eval_at("wiggle(3, 2)", &looping(0.0)).unwrap();
    let end = eval_at("wiggle(3, 2)", &looping(2.0)).unwrap();
    assert!((start - end).abs() < 1e-9);
    assert!(start.abs() <= 3.0);
}

#[test]
fn usage_and_constants_are_reported() {
    assert_eq!(compile("5").unwrap().constant(), Some(5.0));
    let program = compile("on2(t * 2)").unwrap();
    assert_eq!(program.constant(), None);
    assert!(program.uses(Var::Time));
    assert!(!program.uses(Var::Seed));
}

#[test]
fn non_finite_results_are_errors() {
    assert_eq!(eval("1 / 0"), Err(EvalError::NonFinite));
    assert_eq!(eval("log(0)"), Err(EvalError::NonFinite));
    assert_eq!(eval("sqrt(-1)"), Err(EvalError::NonFinite));
}

#[test]
fn compile_errors_carry_kind_and_position() {
    assert_eq!(kind(""), CompileErrorKind::Empty);
    assert_eq!(kind("1 + $"), CompileErrorKind::UnexpectedChar);
    assert_eq!(kind("1 +"), CompileErrorKind::UnexpectedToken);
    assert_eq!(kind("(1"), CompileErrorKind::UnexpectedToken);
    assert_eq!(kind("1 ? 2"), CompileErrorKind::UnexpectedToken);
    assert_eq!(kind("sin"), CompileErrorKind::MissingCall);
    assert_eq!(kind("t(1)"), CompileErrorKind::NotAFunction);
    assert_eq!(
        kind("min(1)"),
        CompileErrorKind::WrongArity {
            min: 2,
            max: 8,
            found: 1
        }
    );
    assert_eq!(
        kind("osc()"),
        CompileErrorKind::WrongArity {
            min: 1,
            max: 3,
            found: 0
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
fn nesting_and_stack_are_bounded() {
    let ok = format!("{}1{}", "(".repeat(MAX_NESTING), ")".repeat(MAX_NESTING));
    assert_eq!(eval(&ok), Ok(1.0));
    let deep = format!(
        "{}1{}",
        "(".repeat(MAX_NESTING + 1),
        ")".repeat(MAX_NESTING + 1)
    );
    assert_eq!(kind(&deep), CompileErrorKind::TooDeep);
    let wide = (0..30).map(|_| "(1+").collect::<String>() + "1" + &")".repeat(30);
    assert_eq!(eval(&wide), Ok(31.0));
    let tall = (0..32).map(|_| "1 + (").collect::<String>() + "1" + &")".repeat(32);
    assert_eq!(kind(&tall), CompileErrorKind::TooComplex);
}

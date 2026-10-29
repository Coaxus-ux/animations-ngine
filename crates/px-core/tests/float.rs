use px_core::floor;

#[test]
fn floor_matches_libm_bit_for_bit() {
    let edges = [
        0.0,
        -0.0,
        0.5,
        -0.5,
        -1.0,
        1.0,
        -1.5,
        2.999_999_999_999_999_6,
        -2.999_999_999_999_999_6,
        4_503_599_627_370_495.5,
        -4_503_599_627_370_495.5,
        4_503_599_627_370_496.0,
        9.0e15,
        -9.0e15,
        1.0e300,
        -1.0e300,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        -f64::EPSILON,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    let sweep = (-4000..4000).map(|i| f64::from(i) * 0.37 + 0.001);
    for value in edges.into_iter().chain(sweep) {
        assert_eq!(
            floor(value).to_bits(),
            libm::floor(value).to_bits(),
            "floor({value})"
        );
    }
    assert!(floor(f64::NAN).is_nan());
}

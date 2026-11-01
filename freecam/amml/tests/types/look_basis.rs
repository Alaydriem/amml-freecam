use alaydriem_freecam::types::LookBasis;

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|axis| (a[axis] - b[axis]).abs() < 1e-5)
}

#[test]
fn yaw_zero_looks_along_positive_z() {
    let basis = LookBasis::from_angles(0.0, 0.0);

    assert!(close(basis.forward, [0.0, 0.0, 1.0]), "{:?}", basis.forward);
    assert!(close(basis.right, [-1.0, 0.0, 0.0]), "{:?}", basis.right);
    assert!(close(basis.up, [0.0, 1.0, 0.0]), "{:?}", basis.up);
}

#[test]
fn the_view_matrix_is_glm_look_at_from_the_origin() {
    let basis = LookBasis::from_angles(90.0, 10.0);

    // glm::lookAt: rows of the rotation are right, up and minus forward, stored column major
    for axis in 0..3 {
        assert!((basis.view[axis][0] - basis.right[axis]).abs() < 1e-6);
        assert!((basis.view[axis][1] - basis.up[axis]).abs() < 1e-6);
        assert!((basis.view[axis][2] + basis.forward[axis]).abs() < 1e-6);
        assert_eq!(basis.view[3][axis], 0.0);
    }
    assert_eq!(basis.view[3][3], 1.0);
}

#[test]
fn the_inverse_undoes_the_view() {
    let basis = LookBasis::from_angles(37.0, -20.0);

    for row in 0..4 {
        for column in 0..4 {
            let product: f32 = (0..4).map(|k| basis.view[k][row] * basis.inverse[column][k]).sum();
            let expected = if row == column { 1.0 } else { 0.0 };
            assert!((product - expected).abs() < 1e-5, "{row} {column} {product}");
        }
    }
}

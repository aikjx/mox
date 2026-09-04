use std::any::TypeId;

#[test]
fn legacy_error_path_is_the_shared_platform_type() {
    let legacy = mox_flow_operator_core::OperatorError::TypeMismatch {
        expected: TypeId::of::<u32>(),
        actual: TypeId::of::<String>(),
    };
    let shared: mox_platform_operator_core::OperatorError = legacy;
    let result: mox_flow_operator_core::Result<()> = Err(shared);
    assert!(matches!(result, Err(mox_platform_operator_core::OperatorError::TypeMismatch { .. })));
}

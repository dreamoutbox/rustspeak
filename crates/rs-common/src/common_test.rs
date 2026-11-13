use crate::CommonError;

#[test]
fn common_error_formats_message() {
    let err = CommonError::Config("missing DATABASE_URL".to_string());
    assert_eq!(
        err.to_string(),
        "invalid configuration: missing DATABASE_URL"
    );
}

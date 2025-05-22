use std::path::PathBuf;

use crate::Runtime;

#[test]
fn test_hps_files() {
    let mut runtime = Runtime::new();

    runtime
        .files
        .add_from_directory(&PathBuf::from("src/tests"));
    runtime.exec_all_files();
    if runtime.any_errors {
        panic!("errors")
    }
}

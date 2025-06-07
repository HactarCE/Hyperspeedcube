use std::path::PathBuf;

use crate::Runtime;

#[test]
fn test_pure_hps() {
    let mut runtime = Runtime::new();

    crate::builtins::define_base_in(&runtime.builtins).expect("error defining built-ins");
    runtime
        .modules
        .add_from_directory(&PathBuf::from("src/tests"));
    runtime.exec_all_files();
    if runtime.diagnostic_count > 0 {
        panic!()
    }
}

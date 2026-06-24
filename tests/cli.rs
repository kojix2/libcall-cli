use std::process::Command;

fn libcall() -> Command {
    Command::new(env!("CARGO_BIN_EXE_libcall"))
}

fn libc_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "System"
    } else {
        "c"
    }
}

#[test]
fn passes_i32_arguments_by_value() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("abs")
        .arg("i32:-42")
        .arg(":i32")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[test]
fn passes_cstr_arguments_as_pointers() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("strlen")
        .arg("hello")
        .arg(":usize")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
}

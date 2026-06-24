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

#[test]
fn flushes_c_stdout_before_printing_return_value() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("puts")
        .arg("hi")
        .arg(":i32")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some("hi"));
    assert!(
        lines.next().is_some(),
        "missing return value line: {stdout}"
    );
}

#[test]
fn supports_i64_return_values() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("atoll")
        .arg("cstr:12345")
        .arg(":i64")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "12345");
}

#[test]
fn infers_i64_for_large_integer_literals() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("labs")
        .arg("3000000000")
        .arg(":isize")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "3000000000");
}

#[test]
fn supports_isize_return_values() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("labs")
        .arg("isize:-5")
        .arg(":isize")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
}

#[test]
fn supports_u64_return_values() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("strlen")
        .arg("abc")
        .arg(":u64")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "3");
}

#[test]
fn supports_non_qsort_void_functions() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("bzero")
        .arg("@4u8:1,2,3,4")
        .arg("usize:4")
        .arg(":void")
        .output()
        .expect("failed to run libcall");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("[0] 4u8 = [0x00, 0x00, 0x00, 0x00]"));
}

#[test]
fn rejects_unsupported_callback_signatures() {
    let output = libcall()
        .arg(format!("-l{}", libc_name()))
        .arg("qsort")
        .arg("@3i32:3,1,2")
        .arg("usize:3")
        .arg("usize:4")
        .arg("'f64(ptr a, ptr b){ return 0.0 }'")
        .arg(":void")
        .output()
        .expect("failed to run libcall");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("Only i32(ptr, ptr) callbacks are currently supported"));
}

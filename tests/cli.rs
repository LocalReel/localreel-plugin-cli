use std::process::Command;

#[test]
fn prints_the_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_localreel-plugin-cli"))
        .arg("--version")
        .output()
        .expect("run the CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "unexpected output: {stdout}"
    );
}

#[test]
fn explains_usage_without_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_localreel-plugin-cli"))
        .output()
        .expect("run the CLI");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("usage:"));
}

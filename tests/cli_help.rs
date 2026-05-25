use std::process::Command;

#[test]
fn help_lists_gotify_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_gotify"))
        .arg("--help")
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("gotify send") && stderr.contains("gotify setup plugin-hook"),
        "help output should list gotify commands, got:\n{stderr}"
    );
}

#[test]
fn version_flag_reports_package_name() {
    let output = Command::new(env!("CARGO_BIN_EXE_gotify"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success(), "--version failed: {output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("gotify-mcp"),
        "version output should mention gotify-mcp, got:\n{stdout}"
    );
}

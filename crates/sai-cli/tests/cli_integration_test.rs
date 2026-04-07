//! End-to-end CLI integration tests using assert_cmd + predicates.

use assert_cmd::Command;
use predicates::prelude::*;

fn sai_code() -> Command {
    Command::cargo_bin("sai-code").unwrap()
}

/// Binary launches and exits with code 0 when stdin reaches EOF immediately.
#[test]
fn exits_zero_on_eof() {
    sai_code().write_stdin("").assert().success().code(0);
}

/// `--help` prints usage information and exits with code 0.
#[test]
fn help_flag_exits_zero() {
    sai_code()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

/// `--version` prints the version string and exits with code 0.
#[test]
fn version_flag_exits_zero() {
    sai_code()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("sai-code"));
}

/// Typing `/exit` causes the agent to exit with code 0.
#[test]
fn exit_command_exits_zero() {
    sai_code().write_stdin("/exit\n").assert().success().code(0);
}

/// Typing `/quit` causes the agent to exit with code 0.
#[test]
fn quit_command_exits_zero() {
    sai_code().write_stdin("/quit\n").assert().success().code(0);
}

/// Empty input lines produce no LLM output and the agent loops back to the prompt.
/// After an empty line followed by EOF, the agent exits cleanly.
#[test]
fn empty_input_exits_cleanly() {
    sai_code().write_stdin("\n\n\n").assert().success().code(0);
}

// Copyright 2022 Martin Pool

//! Test that Nutmeg output is captured within Rust tests.

use std::env;
use std::process::Command;

/// Run the tests in a subprocess and check we don't see leakage on stdout.
#[test]
fn view_in_test_does_not_leak() {
    let cargo = env::var("CARGO").expect("$CARGO isn't set");
    let output = Command::new(&cargo)
        .args(["test", "--test", "captured_in_tests"])
        .output()
        .expect("failed to spawn cargo");
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    println!("stdout:\n{}\nstderr:\n{}\n", stdout_str, stderr_str,);
    assert!(!stdout_str.contains("should be captured"));
    assert!(!stderr_str.contains("should be captured"));
}

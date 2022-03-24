// Copyright 2022 Martin Pool

//! Test that output from the main View constructors is captured inside
//! unit tests.
//!
//! These tests are not expcted to fail, themselves, but in older
//! versions of nutmeg they would leak to the stdout of `cargo test`.
//!
//! `test_output_captured` runs these tests in a subprocess and
//! checks that they don't leak.

use std::io::Write;

#[test]
fn view_stdout_captured() {
    let mut view = nutmeg::View::new(String::new(), nutmeg::Options::default());
    view.update(|model| *model = "stdout progress should be captured".into());
    writeln!(view, "stdout message should be captured").unwrap();
}

#[test]
fn view_stderr_captured() {
    let mut view = nutmeg::View::new(
        String::new(),
        nutmeg::Options::default().destination(nutmeg::Destination::Stderr),
    );
    view.update(|model| *model = "stderr progress should be captured".into());
    writeln!(view, "stderr message should be captured").unwrap();
}

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

use nutmeg::models::DisplayModel;

#[test]
fn view_stdout_captured() {
    let mut view = nutmeg::View::new(DisplayModel("hello"), nutmeg::Options::default());
    view.update(|DisplayModel(message)| *message = "stdout progress should be captured");
    writeln!(view, "stdout message should be captured").unwrap();
}

#[test]
fn view_stderr_captured() {
    let mut view = nutmeg::View::new(
        DisplayModel("initial"),
        nutmeg::Options::default().destination(nutmeg::Destination::Stderr),
    );
    view.update(|model| model.0 = "stderr progress should be captured");
    writeln!(view, "stderr message should be captured").unwrap();
}

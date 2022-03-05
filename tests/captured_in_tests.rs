// Copyright 2022 Martin Pool

//! Test that output from the main View constructors is captured inside
//! unit tests.

use std::io::Write;

#[test]
fn view_stdout_captured() {
    let mut view = nutmeg::View::new(String::new(), nutmeg::ViewOptions::default());
    view.update(|model| *model = "progress should be captured".into());
    writeln!(view, "message should be captured").unwrap();
}

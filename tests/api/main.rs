// Copyright 2022-2023 Martin Pool.

//! API tests for Nutmeg.

use std::io::Write;
use std::time::Duration;

use nutmeg::{Destination, Options};

struct MultiLineModel {
    i: usize,
}

// You can construct options as a static using const fns.
static _SOME_OPTIONS: Options = Options::new()
    .update_interval(Duration::ZERO)
    .print_holdoff(Duration::from_millis(20))
    .destination(Destination::Stderr)
    .fake_clock(false)
    .progress_enabled(true);

// Just the default options are also OK.
static _DEFAULT_OPTIONS: Options = Options::new();

impl nutmeg::Model for MultiLineModel {
    fn render(&mut self, _width: usize) -> String {
        format!("  count: {}\n    bar: {}\n", self.i, "*".repeat(self.i),)
    }
}

#[test]
fn disabled_progress_is_not_drawn() {
    let model = MultiLineModel { i: 0 };
    let options = Options::default()
        .destination(Destination::Capture)
        .progress_enabled(false);
    let view = nutmeg::View::new(model, options);
    let output = view.captured_output();

    for i in 0..10 {
        view.update(|model| model.i = i);
    }
    drop(view);

    assert_eq!(output.lock().unwrap().as_str(), "");
}

#[test]
fn disabled_progress_does_not_block_print() {
    let model = MultiLineModel { i: 0 };
    let options = Options::default()
        .destination(Destination::Capture)
        .progress_enabled(false);
    let mut view = nutmeg::View::new(model, options);
    let output = view.captured_output();

    for i in 0..2 {
        view.update(|model| model.i = i);
        writeln!(view, "print line {i}").unwrap();
    }
    drop(view);

    assert_eq!(
        output.lock().unwrap().as_str(),
        "print line 0\nprint line 1\n"
    );
}

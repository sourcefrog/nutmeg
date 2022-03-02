// Copyright 2022 Martin Pool.

//! API tests for Nutmeg.

use std::io::Write;
use std::time::Duration;

use pretty_assertions::assert_eq;

struct MultiLineModel {
    i: usize,
}

impl nutmeg::Model for MultiLineModel {
    fn render(&mut self, _width: usize) -> String {
        format!("  count: {}\n    bar: {}\n", self.i, "*".repeat(self.i),)
    }
}

#[test]
fn draw_progress_once() {
    let mut out: Vec<u8> = Vec::new();
    let model = MultiLineModel { i: 0 };
    let options = nutmeg::ViewOptions::default();
    let view = nutmeg::View::write_to(model, options, &mut out, 90);

    view.update(|model| model.i = 1);
    drop(view);

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "\x1b[?7l\x1b[0K  count: 1\n    bar: *\x1b[1F\x1b[0J\x1b[?7h"
    );
}

#[test]
fn abandoned_bar_is_not_erased() {
    let mut out: Vec<u8> = Vec::new();
    let model = MultiLineModel { i: 0 };
    let options = nutmeg::ViewOptions::default();
    let view = nutmeg::View::write_to(model, options, &mut out, 90);

    view.update(|model| model.i = 1);
    view.abandon();

    // No erasure commands, just a newline after the last painted view.
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "\x1b[?7l\x1b[0K  count: 1\n    bar: *\n"
    );
}

#[test]
fn suspend_and_resume() {
    struct Model(usize);
    impl nutmeg::Model for Model {
        fn render(&mut self, _width: usize) -> String {
            format!("XX: {}", self.0)
        }
    }
    let mut out: Vec<u8> = Vec::new();
    let model = Model(0);
    let options = nutmeg::ViewOptions::default()
        .update_interval(Duration::ZERO);
    let view = nutmeg::View::write_to(model, options, &mut out, 90);

    for i in 0..=4 {
        if i == 1 {
            view.suspend();
        } else if i == 3 {
            view.resume();
        }
        view.update(|model| model.0 = i);
    }
    view.abandon(); // No erasure commands, just a newline after the last painted view.
                    // * 0 is painted before it's suspended.
                    // * the bar is then erased
                    // * 1 is never painted because the bar is suspended.
                    // * 2 is also updated into the model while the bar is suspended, but then
                    //   it's resumed, so 2 is then painted.
                    // * 3 and 4 are painted in the usual way.
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "\x1b[?7l\x1b[0KXX: 0\
        \x1b[1G\x1b[0J\x1b[?7h\
        \x1b[?7l\x1b[0KXX: 2\
        \x1b[1G\x1b[?7l\x1b[0KXX: 3\
        \x1b[1G\x1b[?7l\x1b[0KXX: 4\n"
    );
}
#[test]
fn disabled_progress_is_not_drawn() {
    let mut out: Vec<u8> = Vec::new();
    let model = MultiLineModel { i: 0 };
    let options = nutmeg::ViewOptions::default().progress_enabled(false);
    let view = nutmeg::View::write_to(model, options, &mut out, 80);

    for i in 0..10 {
        view.update(|model| model.i = i);
    }
    drop(view);

    assert_eq!(String::from_utf8(out).unwrap(), "");
}

#[test]
fn disabled_progress_does_not_block_print() {
    let mut out: Vec<u8> = Vec::new();
    let model = MultiLineModel { i: 0 };
    let options = nutmeg::ViewOptions::default().progress_enabled(false);
    let mut view = nutmeg::View::write_to(model, options, &mut out, 80);

    for i in 0..2 {
        view.update(|model| model.i = i);
        writeln!(view, "print line {}", i).unwrap();
    }
    drop(view);

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "print line 0\nprint line 1\n"
    );
}

/// If output is redirected, it should not be affected by the width of
/// wherever stdout is pointing.
#[test]
fn default_width_when_not_on_stdout() {
    const FORCED_WIDTH: usize = 100;
    struct Model();
    impl nutmeg::Model for Model {
        fn render(&mut self, width: usize) -> String {
            assert_eq!(width, FORCED_WIDTH);
            format!("width={}", width)
        }
    }
    let mut out: Vec<u8> = Vec::new();
    let model = Model();
    let options = nutmeg::ViewOptions::default();
    let view = nutmeg::View::write_to(model, options, &mut out, FORCED_WIDTH);

    view.update(|_model| ());
    drop(view);

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "\x1b[?7l\x1b[0Kwidth=100\x1b[1G\x1b[0J\x1b[?7h"
    );
}

// Copyright 2022 Martin Pool.

//! API tests for Nutmeg.

use std::io::Write;
use std::thread::sleep;
use std::time::{Duration, Instant};

use nutmeg::{Destination, Options, View};

mod identical_output_suppressed;

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
    let model = MultiLineModel { i: 0 };
    let options = Options::default().destination(Destination::Capture);
    let view = nutmeg::View::new(model, options);
    let output = view.captured_output();

    view.update(|model| model.i = 1);
    drop(view);

    assert_eq!(
        output.lock().as_str(),
        "\x1b[?7l\x1b[0J  count: 1\n    bar: *\x1b[1F\x1b[0J\x1b[?7h"
    );
}

#[test]
fn abandoned_bar_is_not_erased() {
    let model = MultiLineModel { i: 0 };
    let view = View::new(model, Options::default().destination(Destination::Capture));
    let output = view.captured_output();

    view.update(|model| model.i = 1);
    view.abandon();

    // No erasure commands, just a newline after the last painted view.
    assert_eq!(
        output.lock().as_str(),
        "\x1b[?7l\x1b[0J  count: 1\n    bar: *\n"
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
    let model = Model(0);
    let options = Options::default()
        .destination(Destination::Capture)
        .update_interval(Duration::ZERO);
    let view = nutmeg::View::new(model, options);
    let output = view.captured_output();

    for i in 0..=4 {
        if i == 1 {
            view.suspend();
        } else if i == 3 {
            view.resume();
        }
        view.update(|model| model.0 = i);
    }
    view.abandon();

    // No erasure commands, just a newline after the last painted view.
    // * 0 is painted before it's suspended.
    // * the bar is then erased
    // * 1 is never painted because the bar is suspended.
    // * 2 is also updated into the model while the bar is suspended, but then
    //   it's resumed, so 2 is then painted.
    // * 3 and 4 are painted in the usual way.
    assert_eq!(
        output.lock().as_str(),
        "\x1b[?7l\x1b[0JXX: 0\
        \x1b[1G\x1b[0J\x1b[?7h\
        \x1b[?7l\x1b[0JXX: 2\
        \x1b[1G\x1b[?7l\x1b[0JXX: 3\
        \x1b[1G\x1b[?7l\x1b[0JXX: 4\n"
    );
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

    assert_eq!(output.lock().as_str(), "");
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
        writeln!(view, "print line {}", i).unwrap();
    }
    drop(view);

    assert_eq!(output.lock().as_str(), "print line 0\nprint line 1\n");
}

/// If output is redirected, it should not be affected by the width of
/// wherever stdout is pointing.
#[test]
fn default_width_when_not_on_stdout() {
    struct Model();
    impl nutmeg::Model for Model {
        fn render(&mut self, width: usize) -> String {
            assert_eq!(width, 80);
            format!("width={}", width)
        }
    }
    let model = Model();
    let options = Options::default().destination(Destination::Capture);
    let view = nutmeg::View::new(model, options);
    let output = view.captured_output();

    view.update(|_model| ());
    drop(view);

    assert_eq!(
        output.lock().as_str(),
        "\x1b[?7l\x1b[0Jwidth=80\x1b[1G\x1b[0J\x1b[?7h"
    );
}

#[test]
fn rate_limiting_with_fake_clock() {
    struct Model {
        draw_count: usize,
        update_count: usize,
    }
    impl nutmeg::Model for Model {
        fn render(&mut self, _width: usize) -> String {
            self.draw_count += 1;
            format!("update:{} draw:{}", self.update_count, self.draw_count)
        }
    }
    let model = Model {
        draw_count: 0,
        update_count: 0,
    };
    let options = Options::default()
        .destination(Destination::Capture)
        .fake_clock(true)
        .update_interval(Duration::from_millis(1));
    let mut fake_clock = Instant::now();
    let view = nutmeg::View::new(model, options);
    view.set_fake_clock(fake_clock);
    let output = view.captured_output();

    // Any number of updates, but until the clock ticks only one will be drawn.
    for _i in 0..10 {
        view.update(|model| model.update_count += 1);
        sleep(Duration::from_millis(10));
    }
    assert_eq!(view.inspect_model(|m| m.draw_count), 1);
    assert_eq!(view.inspect_model(|m| m.update_count), 10);

    // Time passes...
    fake_clock += Duration::from_secs(1);
    view.set_fake_clock(fake_clock);
    // Another burst of updates, and just one of them will be drawn.
    for _i in 0..10 {
        view.update(|model| model.update_count += 1);
        sleep(Duration::from_millis(10));
    }
    assert_eq!(view.inspect_model(|m| m.draw_count), 2);
    assert_eq!(view.inspect_model(|m| m.update_count), 20);

    drop(view);
    assert_eq!(
        output.lock().as_str(),
        "\x1b[?7l\x1b[0Jupdate:1 draw:1\
        \x1b[1G\
        \x1b[?7l\x1b[0Jupdate:11 draw:2\
        \x1b[1G\x1b[0J\x1b[?7h"
    );
}

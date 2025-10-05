// Copyright 2022 Martin Pool.

//! Draw ANSI escape sequences.

// References:
// * <https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797>

use std::borrow::Cow;

#[allow(dead_code)]
pub(crate) const ESC: &str = "\x1b";

pub(crate) const MOVE_TO_START_OF_LINE: &str = "\x1b[1G";

// https://vt100.net/docs/vt510-rm/DECAWM
pub(crate) const DISABLE_LINE_WRAP: &str = "\x1b[?7l";
pub(crate) const ENABLE_LINE_WRAP: &str = "\x1b[?7h";

pub(crate) const CLEAR_TO_END_OF_LINE: &str = "\x1b[0K";
#[allow(dead_code)]
pub(crate) const CLEAR_CURRENT_LINE: &str = "\x1b[2K";
pub(crate) const CLEAR_TO_END_OF_SCREEN: &str = "\x1b[0J";

pub(crate) fn up_n_lines_and_home(n: usize) -> Cow<'static, str> {
    if n > 0 {
        format!("\x1b[{n}F").into()
    } else {
        MOVE_TO_START_OF_LINE.into()
    }
}

#[cfg(windows)]
pub(crate) fn enable_windows_ansi() -> bool {
    crate::windows::enable_windows_ansi()
}

#[cfg(not(windows))]
pub(crate) fn enable_windows_ansi() -> bool {
    true
}

pub(crate) fn insert_codes(rendered: &str, cursor_y: Option<usize>) -> (String, usize) {
    let mut buf = String::with_capacity(rendered.len() + 40);
    buf.push_str(&up_n_lines_and_home(cursor_y.unwrap_or_default()));
    buf.push_str(DISABLE_LINE_WRAP);
    // buf.push_str(CLEAR_TO_END_OF_SCREEN);
    let mut first = true;
    let mut n_lines = 0;
    for line in rendered.lines() {
        if !first {
            buf.push('\n');
            n_lines += 1;
        } else {
            first = false;
        }
        buf.push_str(line);
        buf.push_str(CLEAR_TO_END_OF_LINE);
    }
    buf.push_str(ENABLE_LINE_WRAP);
    (buf, n_lines)
}

#[cfg(test)]
mod test {
    use std::{
        thread::sleep,
        time::{Duration, Instant},
    };

    use super::*;
    use crate::{Destination, Model, Options, View};

    struct MultiLineModel {
        i: usize,
    }

    impl Model for MultiLineModel {
        fn render(&mut self, _width: usize) -> String {
            format!("  count: {}\n    bar: {}\n", self.i, "*".repeat(self.i),)
        }
    }

    #[test]
    fn draw_progress_once() {
        let model = MultiLineModel { i: 0 };
        let options = Options::default().destination(Destination::Capture);
        let view = View::new(model, options);
        let output = view.captured_output();

        view.update(|model| model.i = 1);

        let written = output.lock().unwrap().to_owned();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_string()
                + DISABLE_LINE_WRAP
                + "  count: 1"
                + CLEAR_TO_END_OF_LINE
                + "\n"
                + "    bar: *"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
        );
        output.lock().unwrap().clear();

        drop(view);
        let written = output.lock().unwrap().to_owned();
        assert_eq!(
            written,
            ESC.to_owned() + "[1F" + CLEAR_TO_END_OF_SCREEN + ENABLE_LINE_WRAP
        )
    }

    #[test]
    fn abandoned_bar_is_not_erased() {
        let model = MultiLineModel { i: 0 };
        let view = View::new(model, Options::default().destination(Destination::Capture));
        let output = view.captured_output();

        view.update(|model| model.i = 1);
        view.abandon();

        // No erasure commands, just a newline after the last painted view.
        let written = output.lock().unwrap().to_owned();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned()
                + DISABLE_LINE_WRAP
                + "  count: 1"
                + CLEAR_TO_END_OF_LINE
                + "\n"
                + "    bar: *"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
                + "\n"
        );
    }

    #[test]
    fn rate_limiting_with_fake_clock() {
        struct Model {
            draw_count: usize,
            update_count: usize,
        }
        impl crate::Model for Model {
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
        let view = View::new(model, options);
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
        let written = output.lock().unwrap().to_owned();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned()
                + DISABLE_LINE_WRAP
                + "update:1 draw:1"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
                + MOVE_TO_START_OF_LINE
                + DISABLE_LINE_WRAP
                + "update:11 draw:2"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
                + MOVE_TO_START_OF_LINE
                + CLEAR_TO_END_OF_SCREEN
                + ENABLE_LINE_WRAP
        );
    }

    /// If output is redirected, it should not be affected by the width of
    /// wherever stdout is pointing.
    #[test]
    fn default_width_when_not_on_stdout() {
        struct Model();
        impl crate::Model for Model {
            fn render(&mut self, width: usize) -> String {
                assert_eq!(width, 80);
                format!("width={width}")
            }
        }
        let model = Model();
        let options = Options::default().destination(Destination::Capture);
        let view = View::new(model, options);

        view.update(|_model| ());
        let written = view.take_captured_output();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned()
                + DISABLE_LINE_WRAP
                + "width=80"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
        );
    }

    #[test]
    fn suspend_and_resume() {
        struct Model(usize);
        impl crate::Model for Model {
            fn render(&mut self, _width: usize) -> String {
                format!("XX: {}", self.0)
            }
        }
        let model = Model(0);
        let options = Options::default()
            .destination(Destination::Capture)
            .update_interval(Duration::ZERO);
        let view = View::new(model, options);

        // Paint 0 before it's suspended
        view.update(|model| model.0 = 0);
        let written = view.take_captured_output();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned()
                + DISABLE_LINE_WRAP
                + "XX: 0"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
        );

        // Now suspend; this clears the bar from the screen.
        view.suspend();
        view.update(|model| model.0 = 1);
        let written = view.take_captured_output();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned() + CLEAR_TO_END_OF_SCREEN + ENABLE_LINE_WRAP
        );

        // * 2 is also updated into the model while the bar is suspended, but then
        //   it's resumed, so 2 is then painted.
        view.update(|model| model.0 = 2);
        let written = view.take_captured_output();
        assert_eq!(written, "");

        // Now 2 is painted when resumed.
        view.resume();
        let written = view.take_captured_output();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned()
                + DISABLE_LINE_WRAP
                + "XX: 2"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
        );

        // * 3 and 4 are painted in the usual way.
        view.update(|model| model.0 = 3);
        view.update(|model| model.0 = 4);
        let written = view.take_captured_output();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned()
                + DISABLE_LINE_WRAP
                + "XX: 3"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
                + MOVE_TO_START_OF_LINE
                + DISABLE_LINE_WRAP
                + "XX: 4"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
        );

        let output = view.captured_output();
        view.abandon();
        let written = output.lock().unwrap().to_owned();
        assert_eq!(written, "\n");
    }

    #[test]
    fn identical_output_suppressed() {
        struct Hundreds(usize);

        impl Model for Hundreds {
            fn render(&mut self, _width: usize) -> String {
                format!("hundreds={}", self.0 / 100)
            }
        }

        let options = Options::default()
            .destination(Destination::Capture)
            .update_interval(Duration::ZERO);
        let view = View::new(Hundreds(0), options);

        for i in 0..200 {
            // We change the model, but not in a way that will change what's displayed.
            view.update(|model| model.0 = i);
        }

        // No erasure commands, just a newline after the last painted view.
        let written = view.take_captured_output();
        assert_eq!(
            written,
            MOVE_TO_START_OF_LINE.to_owned()
                + DISABLE_LINE_WRAP
                + "hundreds=0"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
                + MOVE_TO_START_OF_LINE
                + DISABLE_LINE_WRAP
                + "hundreds=1"
                + CLEAR_TO_END_OF_LINE
                + ENABLE_LINE_WRAP
        );
    }
}

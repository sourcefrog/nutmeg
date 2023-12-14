//! Test that Nutmeg avoids redrawing the same text repeatedly.

use std::time::Duration;

use nutmeg::{Destination, Options, View};

struct Hundreds(usize);

impl nutmeg::Model for Hundreds {
    fn render(&mut self, _context: &nutmeg::RenderContext) -> String {
        format!("hundreds={}", self.0 / 100)
    }
}

#[test]
fn identical_output_suppressed() {
    let options = Options::default()
        .destination(Destination::Capture)
        .update_interval(Duration::ZERO);
    let view = View::new(Hundreds(0), options);
    let output = view.captured_output();

    for i in 0..200 {
        // We change the model, but not in a way that will change what's displayed.
        view.update(|model| model.0 = i);
    }
    view.abandon();

    // No erasure commands, just a newline after the last painted view.
    assert_eq!(
        output.lock().unwrap().as_str(),
        "\x1b[?7l\x1b[0Jhundreds=0\x1b[1G\x1b[?7l\x1b[0Jhundreds=1\n"
    );
}

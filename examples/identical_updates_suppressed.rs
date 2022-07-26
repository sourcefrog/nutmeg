//! This model repeatedly generates the same text: it's a counter that
//! only shows the hundreds.
//!
//! Nutmeg avoids redrawing the bar on every update to avoid flickering
//! (on terminals that don't handle this well themselves.)

use std::thread::sleep;
use std::time::Duration;

struct Model {
    i: usize,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        format!("count: {}", self.i / 100)
    }
}

fn main() {
    let options = nutmeg::Options::default();
    let view = nutmeg::View::new(Model { i: 0 }, options);
    for _i in 1..=5000 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(5));
    }
}

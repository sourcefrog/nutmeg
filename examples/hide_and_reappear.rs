//! Example of [nutmeg::View::hide].

use std::thread::sleep;
use std::time::Duration;

struct Model {
    i: usize,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        format!("count: {}", self.i)
    }
}

fn main() {
    let options = nutmeg::Options::default();
    let view = nutmeg::View::new(Model { i: 0 }, options);
    for _i in 1..=5 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(600));

        // bar disappears, but will reappear on the next update.
        view.hide();
        sleep(Duration::from_millis(600));
    }
}

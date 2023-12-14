//! Suspend updates for a while.

use std::thread::sleep;
use std::time::Duration;

struct Model {
    i: usize,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _context: &nutmeg::RenderContext) -> String {
        format!("count: {}", self.i)
    }
}

fn main() {
    let options = nutmeg::Options::default();
    let view = nutmeg::View::new(Model { i: 0 }, options);
    for i in 1..=16 {
        if i == 4 {
            view.suspend();
        } else if i == 10 {
            view.resume();
        }
        view.update(|state| state.i = i);
        sleep(Duration::from_millis(300));
    }
}

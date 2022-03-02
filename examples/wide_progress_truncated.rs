//! Example showing that progress bar that are too wide for the terminal are
//! horizontally truncated, even if `State::render` ignores the advised width.

use std::thread::sleep;
use std::time::Duration;

struct Model {
    i: usize,
    width: usize,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        let mut s = format!("i={} | ", self.i);
        let ii = self.i % self.width;
        for _ in 0..ii {
            s.push('_');
        }
        s.push('🦀');
        for _ in (ii + 1)..self.width {
            s.push('_');
        }
        s
    }
}

fn main() {
    let options = nutmeg::ViewOptions::default().update_interval(Duration::from_millis(50));
    let model = Model { i: 0, width: 120 };
    let view = nutmeg::View::new(model, options);
    for _ in 1..=360 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(100));
    }
    // Should show nothing because progress is disabled
}

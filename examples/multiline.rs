//! Multi-line progress.

use std::thread::sleep;
use std::time::{Duration, Instant};

use yansi::Paint;

struct Model {
    i: usize,
    start: Instant,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        let long_text = self.i.to_string().repeat(40 - self.i);
        format!(
            "  count: {}\n    bar: {}\nelapsed: {:.1}s\n  blink: {}\n   long: {}",
            self.i,
            "*".repeat(self.i),
            self.start.elapsed().as_secs_f32(),
            if self.i % 2 == 0 {
                Paint::red("XXX")
            } else {
                Paint::yellow("XXX")
            },
            long_text,
        )
    }
}

fn main() {
    let options = nutmeg::Options::default().update_interval(Duration::from_millis(50));
    let model = Model {
        i: 0,
        start: Instant::now(),
    };
    let view = nutmeg::View::new(model, options);
    for _i in 1..=40 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(200));
    }
}

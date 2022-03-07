//! Fast updates to the model can be rate-limited in the display.

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
    for update_interval in [20, 50, 100, 250, 1000] {
        println!("update_interval={}ms", update_interval);
        let options =
            nutmeg::Options::default().update_interval(Duration::from_millis(update_interval));
        let view = nutmeg::View::new(Model { i: 0 }, options);
        for _i in 1..=500 {
            view.update(|state| state.i += 1);
            sleep(Duration::from_millis(5));
        }
    }
}

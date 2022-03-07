//! Example of mixing progress updates with printed output.

use std::io::Write;
use std::thread;
use std::time;

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
    let mut view = nutmeg::View::new(Model { i: 0 }, options);
    for i in 1..=25 {
        view.update(|state| state.i += 1);
        if i % 15 == 0 {
            view.message("fizzbuzz\n");
        } else if i % 3 == 0 {
            view.message("fizz\n");
        } else if i % 5 == 0 {
            // Alternatively, you can treat it as a destination for Write.
            writeln!(view, "buzz").unwrap();
        }
        thread::sleep(time::Duration::from_millis(300));
    }
}

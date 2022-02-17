//! Example of mixing progress updates with printed output.

use std::io::Write;
use std::thread;
use std::time;

struct State {
    i: usize,
}

impl nutmeg::State for State {
    fn render<W: std::io::Write>(&self, _width: usize, write_to: &mut W) {
        writeln!(write_to, "count: {}", self.i).unwrap();
    }
}

fn main() {
    let stdout = std::io::stdout();
    let options = nutmeg::ViewOptions::default();
    let mut view = nutmeg::View::new(stdout, State { i: 0 }, options);
    for i in 1..=25 {
        view.update(|state| state.i += 1);
        if i % 15 == 0 {
            writeln!(view, "fizzbuzz").unwrap();
        } else if i % 3 == 0 {
            writeln!(view, "fizz").unwrap();
        }else if i % 5 == 0 {
            writeln!(view, "buzz").unwrap();
        }
        thread::sleep(time::Duration::from_millis(300));
    }
    // Should show nothing because progress is disabled
}

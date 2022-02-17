//! Example of a simple progress bar that counts up.

use std::thread::sleep;
use std::time::Duration;

struct State {
    i: usize,
}

impl nutmeg::Model for State {
    fn render<W: std::io::Write>(&self, _width: usize, write_to: &mut W) {
        writeln!(write_to, "count: {}", self.i).unwrap();
    }
}

fn main() {
    let stdout = std::io::stdout();
    let view = nutmeg::View::new(stdout, State { i: 0 }, nutmeg::ViewOptions::default());
    for _i in 1..=5 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(300));
    }
}

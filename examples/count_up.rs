//! Example of the simplest case: just printing message, no actual
//! progress bar.

use std::thread::sleep;
use std::time::Duration;

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
    let view = nutmeg::View::new(stdout, State { i: 0 }, nutmeg::ViewOptions::default());
    for _i in 1..=5 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(300));
    }
}

//! Example of the simplest case: just printing message, no actual
//! progress bar.

use std::io::Write;

struct State{}

impl nutmeg::State for State {
    fn render<W: std::io::Write>(&self, _width: usize, _write_to: &mut W) {
        // Nothing to do, it never renders.
    }
}

fn main() {
    let stdout = std::io::stdout();
    let mut view = nutmeg::View::new(stdout, State{}, nutmeg::ViewOptions::default());
    for i in 1..=5 {
        writeln!(view, "write line {}", i).unwrap();
    }
}

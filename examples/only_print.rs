//! Example of the simplest case: just printing message, no actual
//! progress bar.

use std::io::Write;

struct Model {}

impl nutmeg::Model for Model {
    fn render(&self, _width: usize) -> String {
        unreachable!("Model::render should never be called, since the progress bar is disabled");
    }
}

fn main() {
    let stdout = std::io::stdout();
    let mut view = nutmeg::View::new(stdout, Model {}, nutmeg::ViewOptions::default());
    for i in 1..=5 {
        writeln!(view, "write line {}", i).unwrap();
    }
}

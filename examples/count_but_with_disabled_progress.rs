//! Example of the simplest case: just printing message, no actual
//! progress bar.

#[allow(unused_imports)]
use std::io::Write;

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
    let mut options = nutmeg::ViewOptions::default();
    options.progress_enabled = false;
    let view = nutmeg::View::new(stdout, State { i: 0 }, options);
    for _i in 1..=5 {
        view.update(|state| state.i += 1);
    }
    // Should show nothing because progress is disabled
}

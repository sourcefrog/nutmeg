//! Example that when the progress bar is disabled, the app can still
//! call `View::update` but nothing is drawn.

#[allow(unused_imports)]
use std::io::Write;

struct Model {
    i: usize,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        format!("count: {}", self.i)
    }
}

fn main() {
    let mut options = nutmeg::ViewOptions::default();
    options.progress_enabled = false;
    let view = nutmeg::View::new(Model { i: 0 }, options);
    for _i in 1..=5 {
        view.update(|state| state.i += 1);
    }
    // Should show nothing because progress is disabled
}

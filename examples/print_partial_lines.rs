//! Partial lines can be printed, and the progress bar does not overwrite them.

use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

struct Model {
    i: usize,
    legal: bool,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        assert!(self.legal);
        format!("progress: {}", self.i)
    }
}

fn zz() {
    sleep(Duration::from_millis(300));
}

fn main() -> io::Result<()> {
    let options = nutmeg::Options::default();
    let model = Model { i: 0, legal: true };
    let mut view = nutmeg::View::new(model, options);
    for i in 1..=5 {
        view.update(|model| model.i += 1);
        zz();
        write!(view, "partial output {}... ", i)?;
        zz();
        view.update(|model| model.i += 1);
        zz();
        write!(view, "more... ")?;
        zz();
        view.update(|model| model.i += 1);
        zz();
        write!(view, "more... ")?;
        zz();
        view.update(|model| model.i += 1);
        zz();
        writeln!(view, "done!")?;
        view.update(|model| model.i += 1);
        zz();
        for _ in 0..4 {
            view.update(|model| model.i += 1);
            zz();
        }
    }
    Ok(())
}

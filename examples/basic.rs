//! The overall example used in README and the library docstring.

use std::io::Write; // to support write!()

// 1. Define a struct holding all the application state necessary to
// render the progress bar.
#[derive(Default)]
struct Model {
    i: usize,
    total: usize,
    last_file_name: String,
}

// 2. Define how to render the progress bar as a String.
impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        format!("{}/{}: {}", self.i, self.total, self.last_file_name)
    }
}

fn main() -> std::io::Result<()> {
    // 3. Create a View when you want to draw a progress bar.
    let mut view = nutmeg::View::new(Model::default(),
        nutmeg::ViewOptions::default());

    // 4. As the application runs, update the model via the view.
    for i in 0..100 {
        view.update(|model| {
            model.i += 1;
            model.last_file_name = format!("file{}.txt", i);
        });
        // 5. Interleave text output lines by writing to the view.
        if i % 10 == 3 {
            writeln!(view, "reached {}", i)?;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // 5. The bar is automatically erased when dropped.
    Ok(())
}

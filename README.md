# nutmeg - an unopinionated progress bar library

https://github.com/sourcefrog/cargo-mutants

[![Tests](https://github.com/sourcefrog/nutmeg/actions/workflows/tests.yml/badge.svg?branch=main&event=push)](https://github.com/sourcefrog/nutmeg/actions/workflows/tests.yml?query=branch%3Amain)
[![docs.rs](https://docs.rs/nutmeg/badge.svg)](https://docs.rs/nutmeg)
[![crates.io](https://img.shields.io/crates/v/nutmeg.svg)](https://crates.io/crates/nutmeg)
[![libs.rs](https://img.shields.io/badge/libs.rs-nutmeg-blue)](https://lib.rs/crates/nutmeg)
![Maturity: Beta](https://img.shields.io/badge/maturity-beta-blue.svg)

Nutmeg draws terminal progress indicators while giving the application complete 
control over their appearance and content.

For more information: <https://docs.rs/nutmeg>

License: MIT

## Example

From `examples/basic.rs`:

```rust
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
```

[![asciicast](https://asciinema.org/a/oPI37ohOY8yhDxomTzHCsR4sw.svg)](https://asciinema.org/a/oPI37ohOY8yhDxomTzHCsR4sw)

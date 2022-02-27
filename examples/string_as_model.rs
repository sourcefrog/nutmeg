//! Use a simple String as a Model, with no need to `impl Model`.

use std::fs::read_dir;
use std::io;
use std::thread::sleep;
use std::time::Duration;

fn main() -> io::Result<()> {
    let options = nutmeg::ViewOptions::default();
    let view = nutmeg::View::new(String::new(), options);
    for p in read_dir(".")? {
        let dir_entry = p?;
        view.update(|model| *model = dir_entry.path().display().to_string());
        sleep(Duration::from_millis(300));
    }
    Ok(())
}

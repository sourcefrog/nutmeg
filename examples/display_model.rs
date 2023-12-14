//! Anything that implements Display can be wrapped in a DisplayModel.

use std::fs::read_dir;
use std::io;
use std::thread::sleep;
use std::time::Duration;

use nutmeg::models::DisplayModel;

fn main() -> io::Result<()> {
    let options = nutmeg::Options::default();
    let model = DisplayModel::new(String::new());
    let view = nutmeg::View::new(model, options);
    for p in read_dir(".")? {
        let dir_entry = p?;
        view.update(|DisplayModel(message)| *message = dir_entry.path().display().to_string());
        sleep(Duration::from_millis(300));
    }
    Ok(())
}

//! Use of [nutmeg::models::BasicModel].

pub fn main() {
    let view = nutmeg::View::new(
        nutmeg::models::BasicModel::new((0, 10), |(a, b)| format!("{}/{} complete", a, b)),
        nutmeg::Options::default(),
    );
    for _i in 0..10 {
        view.update(|model| model.value.0 += 1);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
}

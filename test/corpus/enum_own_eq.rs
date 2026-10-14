// A fieldless enum's own `==`, not JS's `===`, compares it, wherever it's
// asked, `==` and `contains` alike (the second architecture review: five
// predicates said whether `===` is `==`, and only one asked of an `eq` of
// the crate's).

#[derive(Clone, Copy, Debug)]
enum Mode {
    Light,
    Dark,
    Auto,
}

// `Auto` is either.
impl PartialEq for Mode {
    fn eq(&self, other: &Mode) -> bool {
        matches!((self, other), (Mode::Auto, _) | (_, Mode::Auto))
            || matches!((self, other), (Mode::Light, Mode::Light) | (Mode::Dark, Mode::Dark))
    }
}

fn main() {
    println!("{} {}", Mode::Light == Mode::Auto, Mode::Light == Mode::Dark);
    let modes = [Mode::Light, Mode::Auto];
    println!("{} {:?}", modes.contains(&Mode::Dark), modes.iter().position(|m| *m == Mode::Dark));
}

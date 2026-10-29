// A call of another module's function that gives a `bool` is one, as a
// `bool` constant's read is, so a chain of them reads as written,
// `ready && isSupported(name) && (mode < 1 || mode > 9)`, with no `!!` of
// its first two (rd145), as React DOM's ChangeEventPlugin writes
// `canUseDOM && isEventSupported('input') && (..)`.

mod support {
    pub fn is_supported(name: &str) -> bool {
        name.len() > 3
    }
}

mod plugin {
    use super::support::is_supported;

    pub fn input_supported(ready: bool, name: &str, mode: f64) -> bool {
        ready && is_supported(name) && (mode < 1.0 || mode > 9.0)
    }
}

fn main() {
    for (ready, name, mode) in [
        (true, "input", 0.0),
        (true, "in", 0.0),
        (false, "input", 10.0),
        (true, "input", 5.0),
        (true, "input", 11.0),
    ] {
        println!("{}", plugin::input_supported(ready, name, mode));
    }
}

// `{:?}` of a value whose type holds a `fn(&u32)`, `Inspect<_, fn(&u32)>`,
// in a crate whose writes may fail, a writer's `Err`: whether its `Debug`
// may fail is asked of what its type holds, not of `&'a u32`, a type only
// the `fn`'s binder holds, which rustc's trait selection can't be asked of
// and crashed on (#1111).
use std::fmt::{self, Write};

struct Full;

impl Write for Full {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        Err(fmt::Error)
    }
}

#[derive(Debug)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
}

fn show(x: &u32) {
    print!("({x})");
}

fn main() {
    let mut full = Full;
    println!("{:?}", full.write_str("x"));
    let shown = Countdown(3).inspect(show as fn(&u32));
    println!("{shown:?}");
    println!(" {}", shown.sum::<u32>());
}

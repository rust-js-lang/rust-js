//@ library-differs
// In a program that catches panics, what can't panic as it's shown is
// shown as before: a derived `Debug` of numbers, in a `Vec` and an
// `Option`, `{:?}` and `{:#?}`, an enum's, whose `match` has an
// impossible branch that never runs, in a `Result`, and a `Display` that
// only writes, its
// `println!` a line built whole; and a `Display` that may panic, by an
// index, which writes through stdout's line, kept where it's caught.
use std::fmt;
use std::panic;

#[derive(Debug, Clone)]
struct Point {
    x: u32,
    y: u32,
}

struct Plain(u32);

impl fmt::Display for Plain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

struct Indexed(Vec<u32>, usize);

impl fmt::Display for Indexed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "first {} ", self.0[0])?;
        write!(f, "at {}", self.0[self.1])
    }
}

#[derive(Debug)]
enum Mode {
    Off,
    On(u32),
}

#[derive(Debug)]
struct Labeled {
    name: String,
    at: Point,
}

fn main() {
    let points = vec![Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    let caught = panic::catch_unwind(|| println!("{:?} {}", points, Plain(5)));
    println!("plain {}", caught.is_err());
    println!("{:#?}", Some(points[0].clone()));
    println!("{:?}", Labeled { name: "a".to_string(), at: Point { x: 5, y: 6 } });
    let modes: Result<Vec<Mode>, String> = Ok(vec![Mode::Off, Mode::On(2)]);
    println!("{modes:?}");
    let caught = panic::catch_unwind(|| println!("x {} y", Indexed(vec![7, 8], 1)));
    println!("indexed {}", caught.is_err());
    let caught = panic::catch_unwind(|| println!("x {} y", Indexed(vec![7, 8], 5)));
    println!("indexed {}", caught.is_err());
}

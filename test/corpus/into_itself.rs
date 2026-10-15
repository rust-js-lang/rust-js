// `From<T> for T`, std's for every type, and the `Into` it gives: the
// value itself, of a type rust-js knows no other conversion of, and of an
// `impl Into<T>` that is a `T` (ADR 0387). `try!` and `?` convert an error
// so, to its own type.
use std::num::ParseIntError;

#[derive(Debug, PartialEq)]
struct Meters(f64);

fn kept(error: ParseIntError) -> ParseIntError {
    From::from(error)
}

fn parsed(text: &str) -> Result<i32, ParseIntError> {
    let n: i32 = text.parse().map_err(kept)?;
    Ok(n * 2)
}

fn distance() -> impl Into<Meters> {
    Meters(4.5)
}

fn pair<T>(t: T) -> impl Into<[T; 2]>
where
    T: Copy,
{
    [t, t]
}

fn main() {
    println!("{:?} {:?}", parsed("21"), parsed("x").is_err());
    let meters: Meters = distance().into();
    println!("{meters:?} {}", meters == Meters::from(Meters(4.5)));
    println!("{:?}", pair(7).into());
}

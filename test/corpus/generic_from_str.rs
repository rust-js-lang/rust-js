// `T: FromStr` in generic code: `parse` of a `T` is its `FromStr`'s, a
// dictionary each caller gives, std's numbers', `bool`'s, `char`'s and
// `String`'s, or the crate's own. Its `Err` is the impl's, shown by its
// `Debug`.

use std::fmt::Debug;
use std::str::FromStr;

fn read<T: FromStr>(s: &str) -> Option<T> {
    s.parse().ok()
}

fn read_or<T: FromStr>(s: &str, fallback: T) -> T {
    s.trim().parse().unwrap_or(fallback)
}

fn parse_all<T: FromStr>(parts: &[&str]) -> Result<Vec<T>, T::Err> {
    parts.iter().map(|p| p.parse()).collect()
}

fn explain<T>(s: &str) -> String
where
    T: FromStr + Debug,
    T::Err: Debug,
{
    match s.parse::<T>() {
        Ok(v) => format!("{v:?}"),
        Err(e) => format!("error {e:?}"),
    }
}

#[derive(Debug, PartialEq)]
enum Unit {
    Kg,
    Lb,
}

impl FromStr for Unit {
    type Err = String;
    fn from_str(s: &str) -> Result<Unit, String> {
        match s {
            "kg" => Ok(Unit::Kg),
            "lb" => Ok(Unit::Lb),
            _ => Err(format!("unit {s}")),
        }
    }
}

fn main() {
    println!(
        "{:?} {:?} {:?} {:?} {:?}",
        read::<i32>("42"),
        read::<f64>("x"),
        read::<bool>("true"),
        read::<Unit>("kg"),
        read::<u64>("18446744073709551615")
    );
    println!("{} {:?}", read_or("  7 ", 0u8), read_or("nope", Unit::Lb));
    println!("{:?} {}", parse_all::<u16>(&["1", "2"]), parse_all::<i8>(&["1", "300"]).is_err());
    println!("{:?}", parse_all::<Unit>(&["lb", "st"]));
    println!(
        "{} | {} | {} | {}",
        explain::<char>("ab"),
        explain::<Unit>("g"),
        explain::<String>("s"),
        explain::<i64>("-9")
    );
}

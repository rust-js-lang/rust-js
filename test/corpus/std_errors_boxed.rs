// `?` of std's own errors into a `Box<dyn Error>`: a parse error is its
// message (ADR 0063), and its `dyn Error` shows it as Rust does, `{}` the
// message and `{:?}` its `ParseIntError { kind: .. }`.
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct Negative(i64);

impl fmt::Display for Negative {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} is negative", self.0)
    }
}

impl Error for Negative {}

fn parse_positive(s: &str) -> Result<i64, Box<dyn Error>> {
    let n: i64 = s.trim().parse()?;
    if n < 0 {
        return Err(Box::new(Negative(n)));
    }
    Ok(n)
}

fn settings(text: &str) -> Result<(f64, bool, char, u8), Box<dyn Error>> {
    let parts: Vec<&str> = text.split(',').collect();
    let scale: f64 = parts[0].parse()?;
    let on: bool = parts[1].parse()?;
    let mark: char = parts[2].parse()?;
    let wide: i32 = parts[3].parse()?;
    let small = u8::try_from(wide)?;
    Ok((scale, on, mark, small))
}

// Given where an `E: Error` goes, as thiserror's `#[error(transparent)]`
// gives one to its blanket `AsDynError`: std's `Error` of it, its message
// and `Debug`, and no source.
trait AsDyn<'a> {
    fn as_dyn(&self) -> &(dyn Error + 'a);
}

impl<'a, T: Error + 'a> AsDyn<'a> for T {
    fn as_dyn(&self) -> &(dyn Error + 'a) {
        self
    }
}

fn describe<E: Error>(e: &E) -> String {
    format!("{e} | {e:?} | {}", e.source().is_some())
}

fn main() {
    for s in ["42", " 7 ", "-3", "x", "99999999999999999999", ""] {
        match parse_positive(s) {
            Ok(n) => println!("ok {n}"),
            Err(e) => println!("err {e} / {e:?} / {}", e.source().is_none()),
        }
    }
    for text in ["1.5,true,x,7", "z,true,x,7", "1,maybe,x,7", "1,true,xy,7", "1,true,x,300"] {
        match settings(text) {
            Ok(s) => println!("ok {s:?}"),
            Err(e) => println!("err {e} / {e:?}"),
        }
    }
    let err = "x".parse::<i32>().unwrap_err();
    println!("{}", describe(&err));
    println!("{} {}", err.as_dyn(), err.as_dyn().source().is_none());
    println!("{}", describe(&u8::try_from(300i32).unwrap_err()));
    println!("{}", describe(&"maybe".parse::<bool>().unwrap_err()));
}

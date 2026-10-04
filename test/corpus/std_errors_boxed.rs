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
}

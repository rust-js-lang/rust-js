// A writer's `Err(fmt::Error)`, as chrono's formatting returns: `?` carries
// it out of each writer it's in, and a `write!` to a `String` gives it
// back, the string holding what was written before it.
use std::fmt::{self, Display, Write};

struct Hours(u32);

impl Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{:02}h", self.0)
    }
}

struct Span(Hours, Hours);

impl Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)?;
        f.write_str("-")?;
        write!(f, "{}", self.1)
    }
}

fn show(s: &mut String, h: &Hours) -> fmt::Result {
    write!(s, "[{h}]")?;
    s.push('.');
    Ok(())
}

fn main() {
    let mut t = String::from(">");
    let r = write!(t, "{} and {}", Span(Hours(9), Hours(17)), Span(Hours(9), Hours(30)));
    println!("{} {} {t:?}", r.is_err(), r.is_ok());
    let mut u = String::new();
    println!("{:?} {u:?}", write!(u, "{}", Hours(5)));
    let mut v = String::new();
    println!("{:?} {:?} {v:?}", show(&mut v, &Hours(7)), show(&mut v, &Hours(70)));
    println!("{}", Span(Hours(1), Hours(2)));
}

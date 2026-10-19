// `map_err(f)` of a `fmt::Result` that may be an `Err`, a writer's that
// fails when it's full (ADR 0187): `Ok(())`, or `Err(f(error))`, a
// `Result` of its own, as arrayvec's `ArrayString::try_from` makes one.
// And `map(f)`: `Ok(f(()))`, or the `Err`, writeable's `.map(Ok)`.
use std::fmt::{self, Write};

#[derive(Debug)]
struct Full<T>(T, usize);

struct Small {
    text: String,
    cap: usize,
}

impl Write for Small {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.text.len() + s.len() > self.cap {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

fn make(cap: usize, args: fmt::Arguments<'_>) -> Result<String, Full<fmt::Error>> {
    let mut v = Small { text: String::new(), cap };
    v.write_fmt(args).map_err(|e| Full(e, cap))?;
    Ok(v.text)
}

fn counted(cap: usize, text: &str) -> Result<usize, fmt::Error> {
    let mut v = Small { text: String::new(), cap };
    v.write_str(text).map(|()| v.text.len())
}

fn nested(cap: usize) -> Result<Result<(), String>, fmt::Error> {
    let mut v = Small { text: String::new(), cap };
    v.write_str("abc").map(Ok)
}

fn main() {
    println!("{:?}", make(8, format_args!("{}-{}", 1, 2)));
    println!("{:?}", make(2, format_args!("{}-{}", 10, 20)));
    println!("{:?} {:?}", counted(8, "four"), counted(2, "four"));
    println!("{:?} {:?}", nested(8), nested(2));
}

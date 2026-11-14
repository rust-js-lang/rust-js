// A closure that gives a `&mut` it's given to a generic writer function's
// `&mut T`, which generic code holds as a box: given `$cellOf(p)`, as a
// call of it gives it, printed and written into a writer a piece at a time
// (#1714, #1759).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn bumped<T: fmt::Display>(f: &mut fmt::Formatter<'_>, n: &mut T) -> fmt::Result {
    write!(f, "<{}>", n)
}

struct Point(i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

struct Via(i32);

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bump = |a0, a1| bumped(a0, a1);
        let mut p = Point(self.0);
        bump(f, &mut p)
    }
}

fn main() {
    println!("{}", Via(3));
    let mut log = Log(Vec::new());
    write!(log, "[{}]", Via(-4)).unwrap();
    println!("{:?}", log.0);
}

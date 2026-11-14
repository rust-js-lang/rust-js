//! Writes the library's generic types into a writer of its own, directly
//! and through generic code, a piece at a time, each holding a std value,
//! a type of its own and one nested in another; and formats them plainly,
//! of other arguments, a generic caller's and a `dyn`'s too.
use dep::{Either, Small, Tagged, Var};
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Point(i32, i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl fmt::Pointer for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

impl fmt::LowerHex for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("0x")?;
        }
        write!(f, "{:x}", self.1)
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0 + self.1)
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{t}]")
}

fn text<T: fmt::Display>(t: T) -> String {
    format!("{}", Either::<T, u8>::Left(t))
}

pub fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{}", Either::<u8, &str>::Left(7)).unwrap();
    own(&mut log, &Either::<u8, Point>::Right(Point(1, 2))).unwrap();
    write!(log, "{:?}", Either::<Point, bool>::Left(Point(3, 4))).unwrap();
    write!(log, "{}", Small::<_, 4>::of(vec![1.5, 2.0])).unwrap();
    write!(log, "{:?}", Tagged { tag: "t", value: 'x' }).unwrap();
    write!(log, "{}", Either::<Either<u8, char>, u8>::Left(Either::Right('y'))).unwrap();
    println!("{:?}", log.0);
    let mut log = Log(Vec::new());
    write!(log, "{:x}|{:#x}|{:p}", Var(&255u32), Var(&Point(0, 26)), Var(&Point(9, 0))).unwrap();
    println!("{:?}", log.0);
    println!("{:>6x} {:p}", Var(&Point(1, 171)), Var(&Point(2, 0)));
    println!("{}", Either::<String, u8>::Left(String::from("s")));
    println!("{:?}", Either::<u8, String>::Right(String::from("q")));
    println!("{}", text(Point(5, 6)));
    println!("{}", text(-3i64));
    let shown: Box<dyn fmt::Display> = Box::new(Small::<Point, 2>::of(vec![Point(0, 1)]));
    println!("{shown}");
    println!("{:?}", Tagged { tag: "u", value: Point(2, 2) });
    println!("{}", dep::shown(9));
}

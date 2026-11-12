// A writer that prints as it formats (#1490), shown inside a std value's
// `Debug`, a `Vec`'s, an `Option`'s, a tuple's, an array's or a map's:
// Rust's stdout is a `LineWriter`, so each print lands in the line where
// it ran, between the value's brackets and commas, `[!D(1), !D(3)]`. The
// JS makes the value's text whole before it reaches the line, so what's
// printed meanwhile is marked in its writer's text and put back in its
// place as the text reaches the line (#1521). In `{:#?}`, std's
// `PadAdapter` indents a part's lines as its writer writes them, which a
// print made first doesn't go through, `!    D(1),`; a `println!` inside a
// writer ends stdout's line, not the adapter's. And a `format!` of one
// prints as `print!` does, before the line printed next.
use std::collections::BTreeMap;
use std::fmt;

struct D(u32);

impl fmt::Debug for D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("!");
        write!(f, "D({})", self.0)
    }
}

/// Prints between its writes, a whole line too.
struct Two(u32);

impl fmt::Debug for Two {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a")?;
        print!("<{}>", self.0);
        write!(f, "b\nc")?;
        println!("[{}]", self.0);
        write!(f, "d")
    }
}

/// Shows a `Vec` of `D`s inside its own text.
struct Outer(Vec<D>);

impl fmt::Display for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("(");
        write!(f, "outer {:?}", self.0)?;
        print!(")");
        write!(f, " end")
    }
}

fn main() {
    let v = vec![D(1), D(3), D(5)];
    println!("vec {:?}", v);
    let s = format!("{:?}", v);
    println!("format {s}");
    println!("opt {:?} {:?}", Some(D(2)), (D(4), 6));
    println!("[{:>8?}]", Some(D(7)));
    println!("{:?}", vec![Some(D(8)), None]);
    println!("{:#?}", vec![D(1), D(2)]);
    println!("{:#?}", (D(3), Some(D(4))));
    println!("{:?}", vec![Two(1), Two(2)]);
    println!("{:#?}", vec![Two(3)]);
    let m: BTreeMap<u8, D> = [(1, D(5)), (2, D(6))].into_iter().collect();
    println!("{m:?}");
    println!("{:?}", [[D(7)], [D(8)]]);
    println!("{} and {:?}", Outer(vec![D(9), D(10)]), [D(11)]);
    print!("held ");
    println!("{:?}", Some(D(12)));
}

// A writer that prints as it formats (#1490), shown by a derived `Debug`'s
// field or a `Debug` builder's step, of a struct, a tuple struct, an enum's
// variants, nested, a list, a set and a map, `{:?}` and `{:#?}`: each print
// lands in the line where Rust's stdout puts it, `Holder { grid: ]drop
// 0Grid(1), n: 5 }`. The JS shows each field whole before the struct's own
// text, so what's printed meanwhile is marked in the field's text and put
// back in its place as the text reaches the line, and `{:#?}`'s indent goes
// after a print made at a line's start, as std's `PadAdapter` writes it
// (#1521). Was refused (#1495). And a `format!` of one prints as `print!`
// does, before the line printed next.
use std::fmt;

struct Grid(u32);

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("]drop {}", self.0 - 1);
        write!(f, "Grid({})", self.0)
    }
}

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

#[derive(Debug)]
struct Holder {
    grid: Grid,
    n: u32,
}

#[derive(Debug)]
struct Pair(Grid, Two);

#[derive(Debug)]
enum Choice {
    One(Grid),
    Named { two: Two, after: u8 },
    Nothing,
}

#[derive(Debug)]
struct Nested {
    holder: Holder,
    list: Vec<Grid>,
    choice: Option<Choice>,
}

struct Built(Vec<u32>);

impl fmt::Debug for Built {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Built")
            .field("first", &Grid(self.0[0]))
            .field("two", &Two(self.0[1]))
            .finish()?;
        print!("|");
        f.debug_list().entries(self.0.iter().map(|&n| Grid(n))).finish()?;
        f.debug_tuple("T").field(&Two(9)).field(&self.0.len()).finish()?;
        f.debug_set().entry(&Grid(4)).finish()?;
        f.debug_map().entry(&"k", &Grid(5)).entry(&Two(6), &1u8).finish()
    }
}

fn main() {
    println!("{:?}", Holder { grid: Grid(1), n: 5 });
    println!("{:#?}", Holder { grid: Grid(1), n: 5 });
    println!("{:?} {:?}", Pair(Grid(2), Two(3)), Choice::Nothing);
    println!("{:#?}", Pair(Grid(2), Two(3)));
    println!("{:?}", Choice::One(Grid(7)));
    println!("{:#?}", Choice::Named { two: Two(8), after: 1 });
    let nested = Nested {
        holder: Holder { grid: Grid(10), n: 11 },
        list: vec![Grid(12), Grid(13)],
        choice: Some(Choice::Named { two: Two(14), after: 2 }),
    };
    println!("{nested:?}");
    println!("{nested:#?}");
    println!("{:?}", Built(vec![20, 21, 22]));
    println!("{:#?}", Built(vec![23, 24]));
    let s = format!("{:?}", Holder { grid: Grid(30), n: 31 });
    println!("format {s}");
    let s = format!("{:#?}", Built(vec![32, 33]));
    println!("format {s}");
    print!("held ");
    println!("{:>4?}", Pair(Grid(40), Two(41)));
}

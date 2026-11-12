// A derived `Debug`, or a `Debug` builder, of a writer that writes through
// stdout's line (#1490), one of whose parts prints and then panics, the
// panic caught: Rust's stdout keeps what the struct wrote before the part,
// `x Holder { first: !Grid(1), second: !Grid(`, as each piece is written as
// it's taken (#1533). Was refused (#1530). `{:#?}` too, the part's lines
// indented as std's `PadAdapter` writes them, what it wrote before its
// panic among them.
use std::fmt;
use std::panic::catch_unwind;

struct Grid(u32);

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("!");
        write!(f, "Grid(\n")?;
        if self.0 > 2 {
            panic!("too big");
        }
        write!(f, "{})", self.0)
    }
}

#[derive(Debug)]
struct Holder {
    first: Grid,
    second: Grid,
}

#[derive(Debug)]
struct Pair(Grid, Grid);

#[derive(Debug)]
enum Choice {
    Named { only: Grid },
}

struct Built(u32);

impl fmt::Debug for Built {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for n in 1..=self.0 {
            print!("<{n}>");
            list.entry(&Grid(n));
        }
        list.finish()
    }
}

fn main() {
    let r = catch_unwind(|| println!("x {:?} y", Holder { first: Grid(1), second: Grid(3) }));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:#?} y", Holder { first: Grid(1), second: Grid(3) }));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", Pair(Grid(2), Grid(4))));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:#?} y", Choice::Named { only: Grid(5) }));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", Built(3)));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:#?} y", Built(3)));
    println!(" caught {}", r.is_err());
    println!("x {:#?} y", Holder { first: Grid(1), second: Grid(2) });
}

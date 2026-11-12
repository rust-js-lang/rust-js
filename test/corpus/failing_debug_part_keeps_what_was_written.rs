//@ library-differs
// A derived `Debug` or a `Debug` builder one of whose parts returns `Err`,
// first, in the middle or last, written into a `String`: the write gives
// the `Err`, and the `String` keeps what was written before it, `Holder {
// n: 1, bad: B`, as std's builders write each piece as it's taken and skip
// the steps after the error (#1557). `{:?}` and `{:#?}`, a field, an entry
// and `entries`.
use std::fmt::{self, Write};

struct Bad;

impl fmt::Debug for Bad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B\nb")?;
        Err(fmt::Error)
    }
}

#[derive(Debug)]
struct First {
    bad: Bad,
    n: u8,
}

#[derive(Debug)]
struct Middle {
    n: u8,
    bad: Bad,
    m: u8,
}

#[derive(Debug)]
struct Last(u8, Bad);

#[derive(Debug)]
#[allow(dead_code)]
enum Choice {
    Named { n: u8, bad: Bad },
    Plain,
}

#[derive(Debug)]
struct Wide(u8, u8, u8, u8, u8, u8, Bad);

#[derive(Debug)]
enum Item {
    Fine,
    Failing(Bad),
}

struct Entries(usize);

impl fmt::Debug for Entries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<Item> = (0..3)
            .map(|i| if i == self.0 { Item::Failing(Bad) } else { Item::Fine })
            .collect();
        f.debug_list().entries(items.iter()).finish()
    }
}

struct Steps(usize);

impl fmt::Debug for Steps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut set = f.debug_set();
        for i in 0..3 {
            if i == self.0 {
                set.entry(&Bad);
            } else {
                set.entry(&i);
            }
        }
        set.finish()
    }
}

struct Fields(usize);

impl fmt::Debug for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Fields");
        for (i, name) in ["a", "b", "c"].into_iter().enumerate() {
            if i == self.0 {
                s.field(name, &Bad);
            } else {
                s.field(name, &i);
            }
        }
        s.finish_non_exhaustive()
    }
}

struct Mapped(usize);

impl fmt::Debug for Mapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entry(&1, &if self.0 == 0 { Item::Failing(Bad) } else { Item::Fine })
            .entry(&2, &if self.0 == 1 { Item::Failing(Bad) } else { Item::Fine })
            .finish()
    }
}

fn written(show: impl Fn(&mut String) -> fmt::Result) {
    let mut s = String::new();
    let r = show(&mut s);
    println!("{r:?} {s:?}");
}

fn main() {
    written(|s| write!(s, "{:?}", First { bad: Bad, n: 1 }));
    written(|s| write!(s, "{:#?}", First { bad: Bad, n: 1 }));
    written(|s| write!(s, "{:?}", Middle { n: 1, bad: Bad, m: 2 }));
    written(|s| write!(s, "{:#?}", Middle { n: 1, bad: Bad, m: 2 }));
    written(|s| write!(s, "{:?}", Last(1, Bad)));
    written(|s| write!(s, "{:#?}", Last(1, Bad)));
    written(|s| write!(s, "{:?} {:?}", Choice::Plain, Choice::Named { n: 3, bad: Bad }));
    written(|s| write!(s, "{:#?}", Choice::Named { n: 3, bad: Bad }));
    written(|s| write!(s, "{:?}", Wide(1, 2, 3, 4, 5, 6, Bad)));
    written(|s| write!(s, "{:#?}", Wide(1, 2, 3, 4, 5, 6, Bad)));
    for at in 0..4 {
        written(|s| write!(s, "{:?}", Entries(at)));
        written(|s| write!(s, "{:#?}", Entries(at)));
        written(|s| write!(s, "{:?}", Steps(at)));
        written(|s| write!(s, "{:#?}", Steps(at)));
        written(|s| write!(s, "{:?}", Fields(at)));
        written(|s| write!(s, "{:#?}", Fields(at)));
        written(|s| write!(s, "{:?}", Mapped(at)));
        written(|s| write!(s, "{:#?}", Mapped(at)));
    }
}

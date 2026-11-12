//@ library-differs
// A `Debug` builder in a writer that prints (#1490), whose writer runs
// what prints between the builder's pieces: an `entries` iterator that
// prints as it steps, a field's argument that prints as it's made, a held
// builder stepped in a loop that prints, `entries` of items that print as
// they're dropped, `{:?}` and `{:#?}` of each. Rust's builders write each
// piece as it's taken, `[` when the list is made, each entry's separator
// and text as it's stepped, `]` at `finish`, so each print lands between
// them: `x [<step 1>1<step 2>, 2] y` (#1533). A step after one that
// failed isn't taken, its value never shown, though the writer prints on
// and `entries` steps on; a part's lines are indented as std's
// `PadAdapter` writes them, a part of many lines nested in another; and a
// derived `Debug` of a part that prints writes each field as it's shown.
use std::fmt;
use std::fmt::Write;

struct Stepped(Vec<u32>);

impl fmt::Debug for Stepped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|n| {
                print!("<step {n}>");
                n
            }))
            .finish()
    }
}

struct Field(u32);

impl fmt::Debug for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("a", &{
                print!("<a>");
                self.0
            })
            .field("b", &{
                print!("<b>");
                self.0 + 1
            })
            .finish()
    }
}

struct Tupled(u32);

impl fmt::Debug for Tupled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tupled")
            .field(&{
                print!("<0>");
                self.0
            })
            .finish()
    }
}

struct Unnamed(u32);

impl fmt::Debug for Unnamed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("")
            .field(&{
                print!("<u>");
                self.0
            })
            .finish()
    }
}

struct Held(Vec<u32>);

impl fmt::Debug for Held {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for n in &self.0 {
            print!("<held {n}>");
            if *n > 1 {
                list.entry(n);
            }
        }
        list.finish()
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        print!("<drop {}>", self.0);
    }
}

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}", self.0)
    }
}

struct Dropping(u32);

impl fmt::Debug for Dropping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries((0..self.0).map(Loud)).finish()
    }
}

struct Mapped(Vec<(char, u32)>);

impl fmt::Debug for Mapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| {
                print!("<{k}>");
                (k, v)
            }))
            .finish()
    }
}

struct Twice(Vec<u32>, Vec<u32>);

impl fmt::Debug for Twice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        list.entries(self.0.iter().inspect(|n| print!("<first {n}>")));
        print!("<between>");
        list.entries(self.1.iter().inspect(|n| print!("<second {n}>")));
        list.finish()
    }
}

struct Rest(u32);

impl fmt::Debug for Rest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Rest");
        if self.0 > 0 {
            print!("<field>");
            s.field("n", &self.0);
        }
        print!("<rest>");
        s.finish_non_exhaustive()
    }
}

struct Noisy(u32);

impl fmt::Debug for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("<noisy {}>", self.0);
        write!(f, "N{}", self.0)
    }
}

struct Bad(u32);

impl fmt::Debug for Bad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("<bad {}>", self.0);
        write!(f, "B\nb")?;
        Err(fmt::Error)
    }
}

struct Failing;

impl fmt::Debug for Failing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        list.entry(&Noisy(1));
        print!("<before bad>");
        list.entry(&Bad(2));
        print!("<after bad>");
        list.entry(&Noisy(3));
        list.entries([4, 5].iter().inspect(|n| print!("<step {n}>")).map(|&n| Noisy(n)));
        print!("<finishing>");
        list.finish()
    }
}

struct FailingField;

impl fmt::Debug for FailingField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FailingField")
            .field("bad", &Bad(6))
            .field("noisy", &Noisy(7))
            .finish()
    }
}

struct Outer;

impl fmt::Debug for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Outer")
            .field("inner", &Stepped(vec![1, 2]))
            .field("map", &Mapped(vec![('z', 3)]))
            .finish()
    }
}

#[derive(Debug)]
struct Derived {
    noisy: Noisy,
    pair: (Noisy, u8),
    stepped: Stepped,
}

#[derive(Debug)]
struct Wide(Noisy, u8, u8, u8, u8, u8, Noisy);

#[derive(Debug)]
#[allow(dead_code)]
struct Many {
    a: u8,
    b: Noisy,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
}

fn main() {
    let mut s = String::new();
    let r = write!(s, "{:?}", Failing);
    println!(" {r:?} {s:?}");
    let mut s = String::new();
    let r = write!(s, "{:#?}", Failing);
    println!(" {r:?} {s:?}");
    let mut s = String::new();
    let r = write!(s, "{:?} {:#?}", FailingField, FailingField);
    println!(" {r:?} {s:?}");
    println!("x {:?} y", Outer);
    println!("x {:#?} y", Outer);
    let derived = Derived { noisy: Noisy(1), pair: (Noisy(2), 3), stepped: Stepped(vec![4]) };
    println!("x {derived:?} y");
    println!("x {derived:#?} y");
    println!("x {:?} y", Wide(Noisy(5), 1, 2, 3, 4, 5, Noisy(6)));
    println!("x {:?} y", Many { a: 1, b: Noisy(7), c: 2, d: 3, e: 4, f: 5 });
    println!("x {:#?} y", Many { a: 1, b: Noisy(8), c: 2, d: 3, e: 4, f: 5 });
    println!("x {:?} y", Stepped(vec![1, 2]));
    println!("x {:#?} y", Stepped(vec![1, 2]));
    println!("x {:?} y", Stepped(vec![]));
    println!("x {:#?} y", Stepped(vec![]));
    println!("x {:?} y", Field(1));
    println!("x {:#?} y", Field(1));
    println!("x {:?} y", Tupled(3));
    println!("x {:#?} y", Tupled(3));
    println!("x {:?} {:#?} y", Unnamed(4), Unnamed(5));
    println!("x {:?} y", Held(vec![1, 2, 3]));
    println!("x {:#?} y", Held(vec![1, 2, 3]));
    println!("x {:?} y", Held(vec![1]));
    println!("x {:?} y", Dropping(2));
    println!("x {:#?} y", Dropping(2));
    println!("x {:?} y", Mapped(vec![('a', 1), ('b', 2)]));
    println!("x {:#?} y", Mapped(vec![('a', 1)]));
    println!("x {:?} y", Twice(vec![1], vec![2, 3]));
    println!("x {:?} y", Twice(vec![], vec![4]));
    println!("x {:#?} y", Twice(vec![5], vec![]));
    println!("x {:?} {:?} y", Rest(0), Rest(7));
    println!("x {:#?} y", Rest(8));
    let s = format!("{:?}", Stepped(vec![9, 10]));
    println!("format {s}");
    println!("x {:?} y", vec![Stepped(vec![11]), Stepped(vec![12])]);
}

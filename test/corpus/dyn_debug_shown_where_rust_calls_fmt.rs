// A `dyn Debug` stored or passed is its value with its type's `Debug`, `{
// value, impl }` (ADR 0060, amended, #1335), so its `fmt` runs where Rust
// calls it: each time it's shown, never where it's made, and never where it
// isn't shown. A `{:#?}` or `{:x?}` of one made elsewhere is as pretty or
// as hex as it asks. One shown where it's made, a placeholder's, a
// builder's step's or a derived `Debug`'s field's, is the string it shows.
use std::cell::Cell;
use std::fmt::{self, Debug};

struct Loud<'a> {
    calls: &'a Cell<u32>,
}

impl Debug for Loud<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.calls.set(self.calls.get() + 1);
        println!("fmt called");
        write!(f, "Loud#{}", self.calls.get())
    }
}

// A value with a destructor, which its pair drops through its `impl`.
#[derive(Debug)]
struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

#[derive(Debug)]
struct Point {
    x: i32,
    y: u8,
}

#[derive(Debug)]
struct Holder {
    name: &'static str,
    held: Box<dyn Debug>,
}

struct Listed<'a> {
    parts: Vec<&'a dyn Debug>,
}

impl Debug for Listed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        for part in &self.parts {
            list.entry(part);
        }
        list.finish()
    }
}

// A builder's step given a `&dyn Debug` it makes.
struct Built {
    a: u8,
    b: &'static str,
}

impl Debug for Built {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Built").field("a", &self.a).field("b", &self.b).finish()
    }
}

// A writer that gives a `&dyn Debug` it makes to another function.
struct Wrapped(Point);

impl Debug for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wrapped<{}>", described(&self.0))
    }
}

fn described(d: &dyn Debug) -> String {
    format!("{d:?}")
}

fn show_twice(d: &dyn Debug) -> String {
    format!("{d:?} {d:?}")
}

fn never_shown(_d: &dyn Debug) {}

fn pretty(d: &dyn Debug) -> String {
    format!("{d:#?}")
}

fn hex(d: &dyn Debug) -> String {
    format!("{d:x?} {d:#x?}")
}

fn main() {
    let calls = Cell::new(0);
    let loud = Loud { calls: &calls };
    let d: &dyn Debug = &loud;
    println!("made");
    println!("{d:?}");
    println!("{}", show_twice(&loud));
    never_shown(&loud);
    let boxed: Box<dyn Debug> = Box::new(Loud { calls: &calls });
    println!("boxed");
    println!("{boxed:?}");
    // A `Box<dyn Debug>` made a `&dyn Debug` is the same pair.
    let again: &dyn Debug = &boxed;
    println!("{}", show_twice(again));
    println!("{}", calls.get());

    let p = Point { x: -3, y: 255 };
    println!("{}", pretty(&p));
    println!("{}", hex(&p));
    println!("{}", hex(&vec![10u8, 11]));
    let all: Vec<Box<dyn Debug>> = vec![Box::new(1i8), Box::new(Some("a")), Box::new(Point { x: 1, y: 2 })];
    println!("{all:?}");
    println!("{all:#?}");

    let holder = Holder { name: "h", held: Box::new((1, 'c')) };
    println!("{holder:?}");
    println!("{holder:#?}");
    let listed = Listed { parts: vec![&p, &holder, &1.5f64] };
    println!("{listed:?}");
    println!("{listed:#?}");
    println!("{:?}", Wrapped(Point { x: 7, y: 8 }));
    let built = Built { a: 4, b: "four" };
    println!("{built:?} {}", pretty(&built));
    println!("{:#?}", &p as &dyn Debug);
    {
        let noisy: Box<dyn Debug> = Box::new(Noisy(9));
        println!("{noisy:?} {}", pretty(&*noisy));
    }
    println!("{:x?}", Box::new(31u8) as Box<dyn Debug>);
}

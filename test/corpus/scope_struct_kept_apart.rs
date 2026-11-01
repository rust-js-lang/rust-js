// A struct that isn't a function's scope keeps its methods as its type's
// object (ADR 1043): its value given away or given back, made twice, made
// in a loop or an inner block, its method named as a value, a trait
// implemented for it, `pub`, held by an `Rc` that's counted, as its last
// owner runs a destructor, or a unit struct, `undefined`.
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

struct Given {
    n: u32,
}

impl Given {
    fn get(&self) -> u32 {
        self.n
    }
}

fn show(given: &Given) -> u32 {
    given.get() * 10
}

fn given_away(n: u32) -> u32 {
    let given = Given { n };
    given.get() + show(&given)
}

struct Twice {
    n: u32,
}

impl Twice {
    fn get(&self) -> u32 {
        self.n + 1
    }
}

fn made_twice(n: u32) -> u32 {
    let a = Twice { n };
    let b = Twice { n: n * 2 };
    a.get() + b.get()
}

struct Looped {
    n: Cell<u32>,
}

impl Looped {
    fn bump(&self) -> u32 {
        self.n.set(self.n.get() + 1);
        self.n.get()
    }
}

fn made_in_a_loop() -> u32 {
    let mut total = 0;
    for i in 0..3 {
        let looped = Looped { n: Cell::new(i) };
        looped.bump();
        total += looped.bump();
    }
    total
}

struct Named {
    n: u32,
}

impl Named {
    fn get(&self) -> u32 {
        self.n + 2
    }
}

fn named_as_a_value(n: u32) -> u32 {
    let named = Named { n };
    let get = Named::get;
    get(&named)
}

struct Shown {
    n: u32,
}

impl Shown {
    fn twice(&self) -> u32 {
        self.n * 2
    }
}

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "shown {}", self.n)
    }
}

fn implemented(n: u32) -> String {
    let shown = Shown { n };
    format!("{} {}", shown.twice(), shown)
}

pub struct Public {
    n: u32,
}

impl Public {
    fn get(&self) -> u32 {
        self.n + 3
    }
}

fn public(n: u32) -> u32 {
    let public = Public { n };
    public.get()
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Holder {
    noisy: Noisy,
}

impl Holder {
    fn get(&self) -> u32 {
        self.noisy.0 + 4
    }
}

fn counted(n: u32) -> u32 {
    let holder = Rc::new(Holder { noisy: Noisy(n) });
    let got = holder.get();
    println!("got {got}");
    got
}

struct Nested {
    n: u32,
}

impl Nested {
    fn get(&self) -> u32 {
        self.n + 5
    }
}

fn in_a_block(n: u32) -> u32 {
    let got = {
        let nested = Nested { n };
        nested.get()
    };
    got * 2
}

trait Tag {}

struct Tagged {
    n: u32,
}

impl Tag for Tagged {}

impl Tagged {
    fn get(&self) -> u32 {
        self.n + 6
    }
}

fn tagged(n: u32) -> u32 {
    let tagged = Tagged { n };
    tagged.get()
}

struct Returned {
    n: Cell<u32>,
}

impl Returned {
    fn bump(&self) -> u32 {
        self.n.set(self.n.get() + 7);
        self.n.get()
    }
}

fn returned(n: u32) -> Returned {
    let returned = Returned { n: Cell::new(n) };
    returned.bump();
    returned
}

struct Unit;

impl Unit {
    fn method(&mut self, arg: &str) -> Result<usize, &str> {
        Ok(arg.len())
    }

    fn test(self: &mut Unit) -> Result<usize, &str> {
        let s = format!("abcde");
        let data = match self.method(&s) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };
        Ok(data)
    }
}

fn unit() -> usize {
    let mut unit = Unit;
    assert_eq!(unit.method("one"), Ok(3));
    assert_eq!(unit.test(), Ok(5));
    8
}

fn main() {
    println!("{} {}", given_away(1), given_away(2));
    println!("{} {}", made_twice(1), made_in_a_loop());
    println!("{} {}", named_as_a_value(1), implemented(2));
    println!("{} {}", public(1), counted(2));
    println!("{} {}", in_a_block(3), tagged(4));
    println!("{} {}", returned(5).bump(), unit());
}

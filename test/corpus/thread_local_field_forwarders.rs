// A private function only called, that only passes its parameters on to a
// function a thread-local's value holds, read through `with`, is that
// function, as one of a thread-local's function is the variable (ADR 0610,
// amended, #533): the value is taken apart where it's made, `const { add,
// log: say } = makeSystem(0);`, as alien-signals' module takes its system
// apart. Only those functions read it, so each field holds the function
// the value was made with, which reading it once reads. A function of no
// value gives the field's, which no caller reads. The others stay
// functions.
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

struct System {
    add: Rc<dyn Fn(u32, u32) -> u32>,
    log: Box<dyn Fn(&str)>,
    twice: fn(u32) -> u32,
    first: &'static dyn Fn(&[u32]) -> Option<u32>,
}

fn twice(n: u32) -> u32 {
    n * 2
}

fn first(items: &[u32]) -> Option<u32> {
    items.first().copied()
}

fn make_system(offset: u32) -> System {
    System {
        add: Rc::new(move |a, b| a + b + offset),
        log: Box::new(|s| println!("log: {s}")),
        twice,
        first: &first,
    }
}

thread_local! {
    static SYSTEM: System = make_system(0);
}

fn add(a: u32, b: u32) -> u32 {
    SYSTEM.with(|system| (system.add)(a, b))
}

fn say(s: &str) {
    SYSTEM.with(|system| (system.log)(s));
}

fn double(n: u32) -> u32 {
    SYSTEM.with(|system| (system.twice)(n))
}

fn head(items: &[u32]) -> Option<u32> {
    SYSTEM.with(|system| (system.first)(items))
}

struct Adder {
    add: Rc<dyn Fn(u32, u32) -> u32>,
}

fn adder(by: u32) -> Adder {
    Adder {
        add: Rc::new(move |a, b| a * by + b),
    }
}

// A value whose cell is read besides through the function.
struct Counted {
    calls: Cell<u32>,
    add: Rc<dyn Fn(u32, u32) -> u32>,
}

thread_local! {
    static COUNTED: Counted = Counted {
        calls: Cell::new(0),
        add: adder(100).add,
    };
}

fn counted_add(a: u32, b: u32) -> u32 {
    COUNTED.with(|counted| (counted.add)(a, b))
}

fn count() -> u32 {
    COUNTED.with(|counted| {
        counted.calls.set(counted.calls.get() + 1);
        counted.calls.get()
    })
}

// Read besides through its function.
thread_local! {
    static SHARED: Adder = adder(2);
}

fn shared_add(a: u32, b: u32) -> u32 {
    SHARED.with(|shared| (shared.add)(a, b))
}

fn shared_twice(a: u32) -> u32 {
    SHARED.with(|shared| (shared.add)(a, 2))
}

// A function named as a value, whose identity and length are its own.
thread_local! {
    static NAMED: Adder = adder(3);
}

fn named_add(a: u32, b: u32) -> u32 {
    NAMED.with(|named| (named.add)(a, b))
}

// Of no value, which leaves out what the field gives, inside its closure:
// taken apart, as no caller reads what it gives.
thread_local! {
    static QUIET: Adder = adder(4);
}

fn quiet_add(a: u32, b: u32) {
    QUIET.with(|quiet| {
        (quiet.add)(a, b);
    });
}

// And outside it.
thread_local! {
    static LOUD: Adder = adder(5);
}

fn loud_add(a: u32, b: u32) {
    LOUD.with(|loud| (loud.add)(a, b));
}

// A method of the value, which its call is given.
struct Scaled {
    by: u32,
}

impl Scaled {
    fn scale(&self, n: u32) -> u32 {
        n * self.by
    }
}

thread_local! {
    static SCALED: Scaled = Scaled { by: 3 };
}

fn scale(n: u32) -> u32 {
    SCALED.with(|scaled| scaled.scale(n))
}

// A function behind a `Deref` of the crate's, which runs at each call, a
// call its JS shows.
struct Logged(Rc<dyn Fn(u32, u32) -> u32>);

impl Deref for Logged {
    type Target = dyn Fn(u32, u32) -> u32;

    fn deref(&self) -> &Self::Target {
        println!("deref");
        &*self.0
    }
}

struct Held {
    add: Logged,
}

thread_local! {
    static HELD: Held = Held {
        add: Logged(adder(6).add),
    };
}

fn held_add(a: u32, b: u32) -> u32 {
    HELD.with(|held| (held.add)(a, b))
}

// One kept out of line.
thread_local! {
    static APART: Adder = adder(8);
}

#[inline(never)]
fn apart_add(a: u32, b: u32) -> u32 {
    APART.with(|apart| (apart.add)(a, b))
}

// One its module exports.
thread_local! {
    static OPEN: Adder = adder(9);
}

pub fn open_add(a: u32, b: u32) -> u32 {
    OPEN.with(|open| (open.add)(a, b))
}

// One another module reads.
thread_local! {
    static WIDE: Adder = adder(10);
}

fn wide_add(a: u32, b: u32) -> u32 {
    WIDE.with(|wide| (wide.add)(a, b))
}

mod inner {
    pub(super) fn wide_add_one(a: u32) -> u32 {
        super::WIDE.with(|wide| (wide.add)(a, 1))
    }
}

// Called by thread-locals made where they're first used, one written
// before it and one after: taken apart, each read after the value's made.
thread_local! {
    static BEFORE: Cell<u32> = Cell::new(early_add(2, 3));
    static EARLY: Adder = adder(11);
    static AFTER: Cell<u32> = Cell::new(early_add(4, 5));
}

fn early_add(a: u32, b: u32) -> u32 {
    EARLY.with(|early| (early.add)(a, b))
}

fn main() {
    say("start");
    println!("{} {} {:?} {:?}", add(1, 2), double(21), head(&[7, 8]), head(&[]));
    println!("{} {}", counted_add(1, 2), count());
    println!("{} {}", shared_add(3, 4), shared_twice(5));
    let apply: fn(u32, u32) -> u32 = named_add;
    println!("{}", apply(9, 4));
    quiet_add(1, 1);
    loud_add(2, 2);
    println!("{}", scale(5));
    println!("{}", held_add(1, 2));
    println!("{}", apart_add(1, 2));
    println!("{}", open_add(1, 2));
    println!("{} {}", wide_add(1, 2), inner::wide_add_one(3));
    println!("{} {} {}", BEFORE.get(), AFTER.get(), early_add(1, 1));
}

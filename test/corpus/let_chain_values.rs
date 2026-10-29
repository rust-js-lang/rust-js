// A let chain of one binding used as a value, a `let`'s, a function's and a
// thread-local's, each branch one value: its first tests, then what it binds
// only where they hold, read once, then one branch (ADR 0870).
use std::cell::Cell;

const DEBUG: bool = true;

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
    // Made where the module loads: std's code alone. Its binding the
    // module's, named apart from the thread-local of its name.
    static OFFSET: Box<dyn Fn(u32) -> u32> =
        if DEBUG && let Some(offset) = "x40".find('4') {
            Box::new(move |x| x + offset as u32)
        } else {
            Box::new(|x| x)
        };
    static LIMIT: usize = if DEBUG && let Some(limit) = "limit".find('m') { limit } else { 9 };
    // Of a name another's made the module's already.
    static LOWER: usize = if DEBUG && let Some(limit) = "lim".find('i') { limit } else { 8 };
    // Of a global's name, which the module's other code reads.
    static LOGGED: usize = if DEBUG && let Some(console) = "log".find('g') { console } else { 0 };
    // A test of what's no `Option`, `Ok`'s, which `undefined` can't be
    // asked: the chain's two `if`s.
    static BASE: Box<dyn Fn(u32) -> u32> =
        if DEBUG && let Ok(n) = "40".parse::<u32>() {
            Box::new(move |x| x + n)
        } else {
            Box::new(|x| x)
        };
    // Of the crate's code, made as its module loads too (ADR 1006,
    // amended): `main` reads it first.
    static SCALE: Box<dyn Fn(u32) -> u32> =
        if flag("scale", true) && let Some(k) = lookup("scale", 3) {
            Box::new(move |x| x * k)
        } else {
            Box::new(|x| x)
        };
}

fn flag(name: &str, on: bool) -> bool {
    println!("flag {name} {on}");
    on
}

fn lookup(name: &str, n: u32) -> Option<u32> {
    CALLS.with(|c| c.set(c.get() + 1));
    println!("lookup {name} {n}");
    if n > 1 { Some(n) } else { None }
}

fn words(n: u32) -> Option<Vec<String>> {
    println!("words {n}");
    (n > 0).then(|| (0..n).map(|i| format!("w{i}")).collect())
}

pub fn doubled(on: bool, n: u32) -> u32 {
    let v = if flag("doubled", on) && let Some(k) = lookup("doubled", n) { k * 2 } else { 7 };
    v + 1
}

pub fn returned(on: bool, n: u32) -> u32 {
    if flag("returned", on) && let Some(k) = lookup("returned", n) { k + 100 } else { n }
}

pub fn either(a: bool, b: bool, n: u32) -> u32 {
    let v = if a && flag("either", b) && let Some(k) = lookup("either", n) { k } else { 0 };
    v * 3
}

// A payload never falsy, tested by its truth.
pub fn counted(on: bool, n: u32) -> usize {
    let count = if on && let Some(words) = words(n) { words.len() } else { 99 };
    count + 1
}

// The other branch reads a variable of the chain's binding's name.
pub fn shadowed(on: bool, n: u32) -> u32 {
    let k = n + 1000;
    let v = if on && let Some(k) = lookup("shadowed", n) { k } else { k };
    v + k
}

// A branch that does something first stays the chain's two `if`s.
pub fn logged(on: bool, n: u32) -> u32 {
    let v = if on && let Some(k) = lookup("logged", n) {
        println!("found {k}");
        k
    } else {
        5
    };
    v + 1
}

// A test `undefined` can't be asked, of an `Ok`: the chain's two `if`s,
// `n.TAG` read only where `on` holds.
pub fn parsed(on: bool, s: &str) -> u32 {
    let v = if on && let Ok(n) = s.parse::<u32>() { n } else { 0 };
    v + 1
}

// A branch's value of branches of its own stays an `if` of it.
pub fn graded(on: bool, n: u32) -> u32 {
    let v = if on && let Some(k) = lookup("graded", n) {
        if k > 3 { 1 } else { 2 }
    } else {
        0
    };
    v + 10
}

// A `return` the program wrote is a statement.
pub fn early(on: bool, n: u32) -> u32 {
    if on && let Some(k) = lookup("early", n) {
        return k;
    }
    n + 1
}

// `None` of one branch is that branch's guard.
pub fn maybe(on: bool, n: u32) -> Option<u32> {
    if on && let Some(k) = lookup("maybe", n) { Some(k + 1) } else { None }
}

pub struct Settings {
    pub width: Option<u32>,
}

// A field, read again in the one test.
pub fn width(on: bool, settings: &Settings) -> u32 {
    let w = if on && let Some(w) = settings.width { w * 2 } else { 1 };
    w + 1
}

fn main() {
    let scaled = SCALE.with(|f| f(5));
    println!("{}", OFFSET.with(|f| f(2)));
    println!("{}", LIMIT.with(|l| *l));
    println!("{}", LOWER.with(|l| *l));
    println!("{}", LOGGED.with(|l| *l));
    println!("{}", BASE.with(|f| f(1)));
    println!("{scaled}");
    println!("{}", SCALE.with(|f| f(6)));
    for (on, n) in [(true, 3), (true, 1), (false, 3)] {
        println!("doubled {}", doubled(on, n));
        println!("returned {}", returned(on, n));
        println!("counted {}", counted(on, n));
        println!("shadowed {}", shadowed(on, n));
        println!("logged {}", logged(on, n));
        println!("graded {}", graded(on, n + 2));
        println!("early {}", early(on, n));
        println!("maybe {:?}", maybe(on, n));
        let settings = Settings { width: (n > 1).then_some(n) };
        println!("width {}", width(on, &settings));
    }
    println!("parsed {} {} {}", parsed(true, "41"), parsed(true, "x"), parsed(false, "41"));
    for (a, b) in [(true, true), (true, false), (false, true)] {
        println!("either {}", either(a, b, 4));
    }
    println!("counted {}", counted(true, 0));
    println!("calls {}", CALLS.with(|c| c.get()));
}

// Oracle B's own case of std's types and closures (ADR 1204): a `Result`,
// an array, a closure and a `fn` pointer, a `dyn` trait and a `dyn Error`,
// a `Cell` and two `RefCell`s, a plain and a counted `Rc`, iterators given
// back, and a struct of a cell field, each made by a function JS calls for
// its result's shape alone.
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::rc::Rc;

pub trait Area {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Area for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[derive(Debug)]
pub struct Oops;

impl fmt::Display for Oops {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "oops")
    }
}

impl Error for Oops {}

pub struct Tally {
    pub hits: Cell<u32>,
    pub name: String,
}

pub fn checked(n: u32) -> Result<u32, String> {
    if n > 1 { Ok(n) } else { Err(format!("{n} is small")) }
}

pub fn pair(n: u32) -> [u32; 2] {
    [n, n.wrapping_add(1)]
}

pub fn adder(n: u32) -> impl Fn(u32) -> u32 {
    move |x| x.wrapping_add(n)
}

pub fn doubler() -> fn(u32) -> u32 {
    |x| x.wrapping_mul(2)
}

pub fn square(n: u32) -> Box<dyn Area> {
    Box::new(Square(n))
}

pub fn failure() -> Box<dyn Error + Send + Sync> {
    Box::new(Oops)
}

pub fn cell(n: u32) -> Cell<u32> {
    Cell::new(n)
}

pub fn listed(n: u32) -> RefCell<Vec<u32>> {
    RefCell::new(vec![n])
}

pub fn shared(n: u32) -> Rc<u32> {
    Rc::new(n)
}

pub fn counted(n: u32) -> Rc<String> {
    let first = Rc::new(n.to_string());
    let second = Rc::clone(&first);
    assert_eq!(Rc::strong_count(&first), 2);
    drop(second);
    first
}

pub fn upto(n: u32) -> impl DoubleEndedIterator<Item = u32> + ExactSizeIterator {
    0..n.min(5)
}

pub fn evens(n: u32) -> impl Iterator<Item = u32> {
    (0..n.min(9)).filter(|x| x % 2 == 0)
}

pub fn letters(text: &str) -> std::str::Chars<'_> {
    text.chars()
}

pub fn tally(n: u32) -> Tally {
    Tally {
        hits: Cell::new(n),
        name: format!("t{n}"),
    }
}

pub fn widened(a: u32, b: u32) -> u64 {
    u64::from(a) + u64::from(b)
}

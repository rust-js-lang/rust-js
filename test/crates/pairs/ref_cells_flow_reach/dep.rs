//! A library whose consumer can reach some of its cells and not others
//! (ADR 0863, amended, #456). It holds a borrow of a `RefCell<Vec<u32>>`
//! of its own while what it's given runs, so that type is counted. A cell
//! of that type its consumer can reach, one an exported function takes or
//! gives, or a `pub` struct holds, is checked as its type is, its private
//! field's too. One only the crate's own code and a constant of its own may
//! be is each its flow says, as in a program: borrowed only for a moment,
//! it's never checked.
use std::cell::RefCell;
use std::rc::Rc;

pub fn around(f: impl Fn(usize)) {
    let mine = RefCell::new(vec![1u32]);
    let held = mine.borrow_mut();
    f(held.len());
}

/// Taken from the consumer, which may hold a borrow of it.
pub fn push(cell: &RefCell<Vec<u32>>) {
    cell.borrow_mut().push(1);
}

/// Given to the consumer.
pub fn made() -> Rc<RefCell<Vec<u32>>> {
    let cell = Rc::new(RefCell::new(Vec::new()));
    cell.borrow_mut().push(5);
    cell
}

/// A `pub` field the consumer may borrow.
pub struct Open {
    pub items: RefCell<Vec<u32>>,
}

pub fn fill(open: &Open) {
    items_of(open).borrow_mut().push(2);
}

fn items_of(open: &Open) -> &RefCell<Vec<u32>> {
    &open.items
}

/// A static the consumer may name.
pub static mut LOG: RefCell<Vec<u32>> = RefCell::new(Vec::new());

pub fn log(n: u32) {
    // SAFETY: JS runs one thread, and nothing else holds a reference to it.
    unsafe { (*&raw const LOG).borrow_mut().push(n) }
}

pub fn logged() -> usize {
    // SAFETY: as `log`'s.
    unsafe { (*&raw const LOG).borrow().len() }
}

/// A private field no function lends, of a value the consumer holds, as
/// JS it gives it to does, whose properties it reaches.
pub struct Tally {
    items: RefCell<Vec<u32>>,
}

impl Tally {
    pub fn new() -> Self {
        Tally {
            items: RefCell::new(Vec::new()),
        }
    }

    pub fn add(&self, n: u32) {
        self.cell().borrow_mut().push(n);
    }

    pub fn total(&self) -> u32 {
        self.cell().borrow().iter().sum()
    }

    fn cell(&self) -> &RefCell<Vec<u32>> {
        &self.items
    }
}

impl Default for Tally {
    fn default() -> Self {
        Self::new()
    }
}

/// No function takes or gives one: only a constant of the crate's own,
/// `&Stock::One(4)`, is of it beside the one `stocked` makes.
pub enum Stock {
    Many(Rc<RefCell<Vec<u32>>>),
    One(u32),
}

fn count(stock: &Stock) -> u32 {
    match stock {
        Stock::Many(items) => items.borrow().iter().sum(),
        Stock::One(n) => *n,
    }
}

pub fn stocked() -> u32 {
    let stock = Stock::Many(Rc::new(RefCell::new(vec![1, 2])));
    if let Stock::Many(items) = &stock {
        items.borrow_mut().push(3);
    }
    count(&stock) + count(&Stock::One(4))
}

/// The same, of a type no borrow of which is counted: its contract lists
/// no type borrowed unchecked, as no consumer's borrow can be of it.
enum Label {
    Many(Rc<RefCell<Vec<String>>>),
    Empty,
}

fn label_count(label: &Label) -> usize {
    match label {
        Label::Many(names) => names.borrow().len(),
        Label::Empty => 0,
    }
}

pub fn labelled() -> usize {
    let label = Label::Many(Rc::new(RefCell::new(Vec::new())));
    if let Label::Many(names) = &label {
        names.borrow_mut().push(String::from("b"));
    }
    label_count(&label) + label_count(&Label::Empty)
}

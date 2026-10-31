// A trait of classes' method that gives a field is that field, read in
// place where it's called (rd304): its borrows are the field's own, never
// checked where none can meet another, `this.items`, and checked where one
// may, as an inherent method's are; one lent elsewhere lends the field.
use std::cell::RefCell;
use std::rc::Rc;

trait Stack {
    fn items(&self) -> &RefCell<Vec<u32>>;

    fn push(&self, n: u32) {
        self.items().borrow_mut().push(n);
    }

    fn total(&self) -> u32 {
        self.items().borrow().iter().sum()
    }
}

trait Ledger {
    fn entries(&self) -> &RefCell<Vec<String>>;

    fn record(&self, entry: &str) -> bool {
        let mut entries = self.entries().borrow_mut();
        entries.push(entry.to_string());
        // Held: a second borrow meets it.
        self.entries().try_borrow().is_ok()
    }

    fn count(&self) -> usize {
        count_of(self.entries())
    }
}

trait Slot {
    fn slot(&self) -> &RefCell<Option<u32>>;

    fn fill(&self, value: u32) -> bool {
        let held = self.slot().borrow();
        // Held: a borrow of the field's own meets it.
        let free = self.slot().try_borrow_mut().is_ok();
        drop(held);
        *self.slot().borrow_mut() = Some(value);
        free
    }
}

fn count_of(cell: &RefCell<Vec<String>>) -> usize {
    cell.borrow().len()
}

#[cfg_attr(rust_js, rust_js::class)]
struct Books {
    items: RefCell<Vec<u32>>,
    entries: RefCell<Vec<String>>,
    slot: RefCell<Option<u32>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Books {
    fn new() -> Self {
        Books {
            items: RefCell::new(vec![1]),
            entries: RefCell::new(Vec::new()),
            slot: RefCell::new(None),
        }
    }

    fn first(&self) -> u32 {
        self.items.borrow()[0]
    }

    fn slotted(&self) -> Option<u32> {
        *self.slot.borrow()
    }
}

impl Stack for Books {
    fn items(&self) -> &RefCell<Vec<u32>> {
        &self.items
    }
}

impl Slot for Books {
    fn slot(&self) -> &RefCell<Option<u32>> {
        &self.slot
    }
}

impl Ledger for Books {
    fn entries(&self) -> &RefCell<Vec<String>> {
        &self.entries
    }
}

struct Shared {
    items: RefCell<Vec<u32>>,
}

// Its `items` gives a field of what an `Rc` holds, not of its own: a
// method of its prototype.
#[cfg_attr(rust_js, rust_js::class)]
struct Tally {
    shared: Rc<Shared>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Tally {
    fn new() -> Self {
        Tally {
            shared: Rc::new(Shared {
                items: RefCell::new(Vec::new()),
            }),
        }
    }
}

impl Stack for Tally {
    fn items(&self) -> &RefCell<Vec<u32>> {
        &self.shared.items
    }

    fn total(&self) -> u32 {
        self.items().borrow().len() as u32 * 100
    }
}

fn main() {
    let books = Books::new();
    books.push(2);
    books.push(3);
    println!("{} {}", books.total(), books.first());
    println!("{} {}", books.record("a"), books.record("b"));
    println!("{}", books.count());
    println!("{:?} {} {:?}", books.slotted(), books.fill(4), books.slotted());
    println!("{}", books.items().borrow().len());
    let tally = Tally::new();
    tally.push(7);
    println!("{} {:?}", tally.total(), tally.items().borrow());
}

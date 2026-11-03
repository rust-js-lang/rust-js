// A `replace` or `take` of a cell whose old value nothing reads, and whose
// drop does nothing, is the write alone, as `*cell.borrow_mut() = v` is:
// React's useActionState, `metadata.action = permalink` of
// `metadata.action.replace(Some(..))` (rd418, ADR 1022, amended). One
// whose old value is read, or whose drop does something, reads it first.
use std::cell::{Cell, RefCell};

struct Action {
    name: Option<String>,
    action: RefCell<Option<String>>,
    count: Cell<u32>,
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

struct Slot {
    held: RefCell<Option<Noisy>>,
}

fn renamed(metadata: &Action, permalink: &str) {
    metadata.action.replace(Some(permalink.to_owned()));
}

fn cleared(metadata: &Action) {
    metadata.action.take();
}

fn counted(metadata: &Action, n: u32) {
    metadata.count.replace(n);
}

fn in_closure(metadata: &Action, permalinks: &[&str]) {
    permalinks.iter().for_each(|permalink| {
        metadata.action.replace(Some(permalink.to_string()));
    });
}

// The old value read: given back.
fn swapped(metadata: &Action, permalink: &str) -> Option<String> {
    metadata.action.replace(Some(permalink.to_owned()))
}

// Named, and read.
fn logged(metadata: &Action, permalink: &str) {
    let old = metadata.action.replace(Some(permalink.to_owned()));
    println!("was {:?}", old);
}

// A drop that does something: the old value dropped after the write.
fn refilled(slot: &Slot, next: &'static str) {
    slot.held.replace(Some(Noisy(next)));
    println!("refilled {next}");
}

fn emptied(slot: &Slot) {
    slot.held.take();
    println!("emptied");
}

fn main() {
    let metadata = Action {
        name: Some("form".to_owned()),
        action: RefCell::new(None),
        count: Cell::new(0),
    };
    renamed(&metadata, "/a");
    println!("{:?} {:?}", metadata.name, metadata.action.borrow());
    renamed(&metadata, "/b");
    println!("{:?}", metadata.action.borrow());
    cleared(&metadata);
    println!("{:?}", metadata.action.borrow());
    counted(&metadata, 3);
    counted(&metadata, 4);
    println!("{}", metadata.count.get());
    in_closure(&metadata, &["/c", "/d"]);
    println!("{:?}", metadata.action.borrow());
    println!("{:?}", swapped(&metadata, "/e"));
    logged(&metadata, "/f");
    println!("{:?}", metadata.action.borrow());
    let slot = Slot {
        held: RefCell::new(Some(Noisy("first"))),
    };
    refilled(&slot, "second");
    refilled(&slot, "third");
    emptied(&slot);
    emptied(&slot);
    println!("{}", slot.held.borrow().is_none());
}

// A scope struct's field made of a local moved into its literal is that
// local only where nothing assigns or reads it again after the move, on
// any path (#2036): one the program assigns again after, `count =
// Cell::new(100)`, or remakes each turn of a loop the literal's in, is
// copied into the field, so the new value never overwrites the field's;
// one moved and never touched again is the field itself; and one assigned
// again after its move and never read.
use std::cell::Cell;
use std::rc::Rc;

struct Reassigned {
    count: Cell<u32>,
    step: u32,
}

impl Reassigned {
    fn tick(self: &Rc<Self>) -> u32 {
        self.count.set(self.count.get() + self.step);
        self.count.get()
    }
}

fn reassigned() -> u32 {
    let mut count = Cell::new(1);
    let step = 2;
    let ticker = Rc::new(Reassigned { count, step });
    let first = ticker.tick();
    count = Cell::new(100);
    first * 1000 + count.get() + ticker.tick()
}

struct Looped {
    count: Cell<u32>,
    step: u32,
}

impl Looped {
    fn tick(self: &Rc<Self>) -> u32 {
        self.count.set(self.count.get() + self.step);
        self.count.get()
    }
}

fn looped() -> u32 {
    let mut total = 0;
    let mut count = Cell::new(0);
    for turn in 0..3 {
        let ticker = Rc::new(Looped { count, step: turn + 1 });
        total += ticker.tick() * 10 + ticker.tick();
        count = Cell::new(total);
    }
    total
}

struct Once {
    count: Cell<u32>,
    step: u32,
}

impl Once {
    fn tick(self: &Rc<Self>) -> u32 {
        self.count.set(self.count.get() + self.step);
        self.count.get()
    }
}

fn moved_once() -> u32 {
    let count = Cell::new(5);
    let step = 3;
    let ticker = Rc::new(Once { count, step });
    ticker.tick() + ticker.tick()
}

struct Unread {
    base: u32,
    count: Cell<u32>,
}

impl Unread {
    fn tick(self: &Rc<Self>) -> u32 {
        self.count.set(self.count.get() + 1);
        self.base * 10 + self.count.get()
    }
}

// Valid Rust that rustc warns on, `base = 100` never read: kept because the
// JS must still be right, the field `base` the 7 it was made of, never the
// local's new value. No shape writes a moved local again without that
// warning but one that reads or borrows it.
#[allow(unused_assignments)]
fn reassigned_unread() -> u32 {
    let mut base = 7;
    let count = Cell::new(0);
    let ticker = Rc::new(Unread { base, count });
    base = 100;
    let first = ticker.tick();
    first * 100 + ticker.tick()
}

fn main() {
    println!("{}", reassigned());
    println!("{}", looped());
    println!("{}", moved_once());
    println!("{}", reassigned_unread());
}

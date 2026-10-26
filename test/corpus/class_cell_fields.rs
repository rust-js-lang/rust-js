// A class's method sets its `Cell` fields on `this` (#359): `self.reads.set(..)`
// is `this.reads = ..`, and a handle it hands on, `bump(&self.reads)`, is
// on the instance's field, named outside the handle's accessors, whose
// `this` is the handle. What Rust does with it is what it does natively.

use std::cell::Cell;

fn bump(count: &Cell<u32>) {
    count.set(count.get() + 1);
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Counted {
    row: i32,
    reads: Cell<u32>,
    last: Cell<Option<i32>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counted {
    pub fn new(row: i32) -> Self {
        Counted {
            row,
            reads: Cell::new(0),
            last: Cell::new(None),
        }
    }

    pub fn row_counted(&self) -> i32 {
        self.reads.set(self.reads.get() + 1);
        self.last.set(Some(self.row));
        self.row
    }

    pub fn row_bumped(&self, times: u32) -> i32 {
        for _ in 0..times {
            bump(&self.reads);
        }
        self.row
    }

    pub fn read_count(&self) -> u32 {
        self.reads.get()
    }

    pub fn last_read(&self) -> Option<i32> {
        self.last.get()
    }
}

fn main() {
    let counted = Counted::new(3);
    println!("{:?}", counted.last_read());
    let total = counted.row_counted() + counted.row_counted() + counted.row_bumped(2);
    println!("{total} {} {:?}", counted.read_count(), counted.last_read());
}

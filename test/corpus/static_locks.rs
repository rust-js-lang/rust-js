// A static `Mutex`, `RwLock` and atomic, std's way to keep global state:
// each use is the one place the static is, changed in place through its
// lock, as Bell numbers' lookup table is.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

struct Table {
    items: Vec<u64>,
}

impl Table {
    const fn new() -> Self {
        Table { items: Vec::new() }
    }
}

static TABLE: RwLock<Table> = RwLock::new(Table::new());
static CALLS: Mutex<u32> = Mutex::new(0);
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn remember(n: u64) -> usize {
    *CALLS.lock().unwrap() += 1;
    let mut table = TABLE.write().unwrap();
    table.items.push(n);
    table.items.len()
}

fn next_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() {
    println!("{} {}", remember(3), remember(4));
    let table = TABLE.read().unwrap();
    println!("{:?} {}", table.items, CALLS.lock().unwrap());
    let ids = [next_id(), next_id(), next_id()];
    println!("{ids:?} {}", NEXT_ID.load(Ordering::SeqCst));
}

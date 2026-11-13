// `{:?}` of std's cells and locks, `RefCell`, `Cell`, `Mutex` and
// `RwLock`, one of whose parts prints and then panics, the panic caught:
// Rust's stdout keeps what the cell wrote before the part, `x RefCell {
// value: !Grid(`, as std's `debug_struct` writes each piece as it's taken
// (#1530). A writer's `Err` keeps it too, in what it gave back. `{:#?}` of a
// lock is a field a line (#1629), and a panic out of a `Mutex`'s part
// poisons it, as the guard std's `Debug` holds drops (#1631).
use std::cell::{Cell, RefCell};
use std::fmt::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, RwLock};

struct Grid(u32);

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("!");
        write!(f, "Grid(")?;
        if self.0 > 2 {
            panic!("too big");
        }
        write!(f, "{})", self.0)
    }
}

struct Failing(u32);

impl fmt::Debug for Failing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failing(")?;
        if self.0 > 2 {
            return Err(fmt::Error);
        }
        write!(f, "{})", self.0)
    }
}

struct Quiet(u32);

impl fmt::Debug for Quiet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 > 2 {
            panic!("quiet");
        }
        write!(f, "Quiet({})", self.0)
    }
}

fn main() {
    let caught = catch_unwind(|| println!("a {:?} b", RefCell::new(Grid(3))));
    println!("caught {}", caught.is_err());
    let cell = RefCell::new(Grid(3));
    let caught = catch_unwind(AssertUnwindSafe(|| println!("c {:#?} d", cell)));
    // The borrow the panic unwound through is given back.
    println!("caught {} borrowed {}", caught.is_err(), cell.try_borrow_mut().is_ok());
    let caught = catch_unwind(|| println!("e {:?} f", vec![RefCell::new(Grid(1)), RefCell::new(Grid(3))]));
    println!("caught {}", caught.is_err());
    let caught = catch_unwind(|| println!("g {:?} {:?} h", Cell::new(1), Mutex::new(Grid(1))));
    println!("caught {}", caught.is_err());

    let lock = Mutex::new(Grid(3));
    let caught = catch_unwind(AssertUnwindSafe(|| println!("i {:?} j", lock)));
    println!("caught {} poisoned {}", caught.is_err(), lock.is_poisoned());
    let lock = RwLock::new(Grid(3));
    let caught = catch_unwind(AssertUnwindSafe(|| println!("k {:#?} l", lock)));
    println!("caught {} poisoned {}", caught.is_err(), lock.is_poisoned());
    let caught = catch_unwind(|| println!("m {:?} n", Some(Mutex::new(Grid(3)))));
    println!("caught {}", caught.is_err());

    // A panic out of a `Mutex`'s part poisons it, where nothing printed.
    let quiet = Mutex::new(Quiet(3));
    let caught = catch_unwind(AssertUnwindSafe(|| format!("{quiet:?}")));
    println!("caught {} poisoned {}", caught.is_err(), quiet.is_poisoned());
    println!("locked {}", quiet.lock().is_err());

    // A writer's `Err`: what came before it kept, and no lock poisoned.
    let mut text = String::new();
    let written = write!(text, "o {:?}", RefCell::new(Failing(3)));
    println!("{written:?} {text:?}");
    let failing = Mutex::new(Failing(3));
    let mut text = String::new();
    let written = write!(text, "p {failing:#?}");
    println!("{written:?} {text:?} poisoned {}", failing.is_poisoned());
    let mut text = String::new();
    let written = write!(text, "q {:?}", RwLock::new(Failing(1)));
    println!("{written:?} {text:?}");

    // `{:#?}` of each, and of one held while it's shown.
    println!("{:#?}", Mutex::new(1));
    println!("{:#?}", RwLock::new(vec![1, 2]));
    println!("{:#?}", Cell::new(3));
    let held = Mutex::new(5);
    let guard = held.lock().unwrap();
    println!("{held:?} {held:#?}");
    drop(guard);
    let cell = RefCell::new(Grid(1));
    let borrowed = cell.borrow_mut();
    let caught = catch_unwind(AssertUnwindSafe(|| println!("r {cell:#?} s")));
    println!("caught {}", caught.is_err());
    drop(borrowed);
}

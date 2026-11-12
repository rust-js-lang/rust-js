// `cycle()` of the crate's iterator whose clone or drop panics drops what
// it holds as std's `Cycle` does (#1585): `Cycle::new` drops what it was
// given where its clone panics; a clone of a `Cycle` a struct holds drops
// the clone of what it started with where the clone of what it steps
// panics; and where the drop of what it steps panics as it's replaced, the
// fresh clone is what it holds, dropped as the panic unwinds, the old one
// not again.
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
    static PANIC_CLONE_AT: Cell<u32> = const { Cell::new(u32::MAX) };
    static PANIC_DROP_OF: Cell<u32> = const { Cell::new(u32::MAX) };
}

struct Steps {
    id: u32,
    left: u32,
}

impl Iterator for Steps {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        Some(self.id)
    }
}

impl Clone for Steps {
    fn clone(&self) -> Self {
        let n = CLONES.with(|c| c.get() + 1);
        CLONES.with(|c| c.set(n));
        if n == PANIC_CLONE_AT.with(Cell::get) {
            panic!("clone {n}");
        }
        println!("clone {} as {}", self.id, self.id * 10 + n);
        Steps {
            id: self.id * 10 + n,
            left: self.left,
        }
    }
}

impl Drop for Steps {
    fn drop(&mut self) {
        println!("drop Steps {}", self.id);
        if self.id == PANIC_DROP_OF.with(Cell::get) {
            panic!("drop {}", self.id);
        }
    }
}

#[derive(Clone)]
struct Held(std::iter::Cycle<Steps>);

fn reset(clone_at: u32, drop_of: u32) {
    CLONES.with(|c| c.set(0));
    PANIC_CLONE_AT.with(|c| c.set(clone_at));
    PANIC_DROP_OF.with(|c| c.set(drop_of));
}

fn main() {
    reset(1, u32::MAX);
    let caught = catch_unwind(|| {
        let _cycled = Steps { id: 7, left: 1 }.cycle();
    });
    println!("new caught {}", caught.is_err());

    reset(3, u32::MAX);
    let caught = catch_unwind(|| {
        let held = Held(Steps { id: 8, left: 1 }.cycle());
        let _again = held.clone();
    });
    println!("clone caught {}", caught.is_err());

    reset(u32::MAX, 9);
    let caught = catch_unwind(|| {
        let mut cycled = Steps { id: 9, left: 1 }.cycle();
        println!("{:?}", cycled.next());
        println!("{:?}", cycled.next());
    });
    println!("replace caught {}", caught.is_err());

    reset(u32::MAX, u32::MAX);
    let taken: Vec<u32> = Steps { id: 5, left: 2 }.cycle().take(5).collect();
    println!("{taken:?}");
}

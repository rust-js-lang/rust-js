// What a module makes where it loads, a thread-local's value, reads another
// of the crate's modules' constants and numbered enum's variants as their
// values: the two modules import each other, and the one loaded first runs
// its load before the other has made its `const`s (ADRs 0031, 0722):
// `kinds`, imported first, imports `state`, which loads first. A
// body run later, a function's, reads them by their names, and so does a
// load of its own module's, made before it.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
enum Phase {
    Early = 1,
    Late = 2,
}

thread_local! {
    static PHASE: Cell<Phase> = Cell::new(Phase::Early);
}

mod kinds {
    #[derive(Clone, Copy, Debug, PartialEq)]
    #[repr(u8)]
    pub enum Status {
        Pending = 3,
        Done = 4,
    }

    pub const LIMIT: u32 = 7;

    pub fn current() -> (Status, u32) {
        (super::state::STATE.get(), super::state::COUNT.get())
    }
}

mod state {
    use super::kinds::{LIMIT, Status};
    use std::cell::Cell;

    thread_local! {
        pub static STATE: Cell<Status> = Cell::new(Status::Pending);
        pub static COUNT: Cell<u32> = Cell::new(LIMIT);
    }

    pub fn finish() {
        STATE.set(Status::Done);
        COUNT.set(LIMIT * 2);
    }
}

fn main() {
    let before = kinds::current();
    state::finish();
    let early = PHASE.get();
    PHASE.set(Phase::Late);
    println!("{before:?} {:?} {early:?} {:?}", kinds::current(), PHASE.get());
}

// A thread-local a test proved `Some` where a closure is made, of a type
// with no interior mutability, read `Some` in the closure through `with`'s
// parameter, React DOM's `localRequestAnimationFrame` of `scheduleTimeout`
// (#1933 step 4 (G)); one made where it's `None` panics when called. And a
// thread-local of a `Cell`, a `RefCell` and a `static mut`, each tested
// before its closure is made and set to `None` after: no test of it holds
// when the closure runs, so its `unwrap()` panics as Rust's does.
use std::cell::{Cell, RefCell};
use std::panic::catch_unwind;

type Scheduled = fn(i32) -> i32;

fn double(x: i32) -> i32 {
    x * 2
}

thread_local! {
    static SCHEDULE: Option<Scheduled> = if "ab".len() > 1 { Some(double) } else { None };
    static MISSING: Option<Scheduled> = if "ab".len() > 5 { Some(double) } else { None };
    static FRAME: Option<Scheduled> = if SCHEDULE.with(Option::is_some) {
        Some(|x| SCHEDULE.with(|schedule| schedule.unwrap())(x) + 1)
    } else {
        None
    };
    static NOT_NONE: Option<Scheduled> = if SCHEDULE.with(Option::is_none) {
        None
    } else {
        Some(|x| SCHEDULE.with(|schedule| schedule.unwrap())(x) + 2)
    };
    static ON_NONE: Option<Scheduled> = if MISSING.with(Option::is_none) {
        Some(|x| MISSING.with(|missing| missing.unwrap())(x))
    } else {
        None
    };
    static SLOT: Cell<Option<Scheduled>> = Cell::new(Some(double));
    static HELD: RefCell<Option<Scheduled>> = RefCell::new(Some(double));
}

static mut CHANGING: Option<Scheduled> = Some(double);

fn slot_reader() -> Option<Scheduled> {
    if SLOT.get().is_some() { Some(|x| SLOT.get().unwrap()(x)) } else { None }
}

fn held_reader() -> Option<Scheduled> {
    if HELD.with_borrow(Option::is_some) {
        Some(|x| HELD.with_borrow(|held| held.unwrap())(x))
    } else {
        None
    }
}

fn changing_reader() -> Option<Scheduled> {
    // SAFETY: one thread reads and writes it, never at once.
    let now = unsafe { CHANGING };
    if now.is_some() {
        // SAFETY: as above.
        Some(|x| (unsafe { CHANGING }).unwrap()(x))
    } else {
        None
    }
}

fn main() {
    let frame = FRAME.with(|frame| frame.unwrap());
    println!("{}", frame(20));
    println!("{}", NOT_NONE.with(|frame| frame.unwrap())(20));
    let on_none = ON_NONE.with(|on_none| on_none.unwrap());
    println!("{}", catch_unwind(|| on_none(1)).is_err());
    let slot = slot_reader().unwrap();
    println!("{}", slot(4));
    SLOT.set(None);
    println!("{}", catch_unwind(|| slot(4)).is_err());
    let held = held_reader().unwrap();
    println!("{}", held(5));
    HELD.with_borrow_mut(|held| *held = None);
    println!("{}", catch_unwind(|| held(5)).is_err());
    let changing = changing_reader().unwrap();
    println!("{}", changing(6));
    // SAFETY: as above.
    unsafe { CHANGING = None };
    println!("{}", catch_unwind(|| changing(6)).is_err());
}

// A `Cell`'s or a thread-local `Cell`'s set never panics where JS drops
// nothing of the value it replaces, and its replace and an `Option`'s take
// never do (#861): a guard that restores an `Option` of a reference or of an
// `Rc` no count is kept of drops in a `finally`. One whose old value has a
// destructor, a `Loud` or an `Rc` of one, still drops after its `try` and as
// a panic unwinds, the old value's drop run where Rust runs it.
use std::cell::Cell;
use std::panic;
use std::rc::Rc;

pub struct Fiber(pub u32);

pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("loud {}", self.0);
    }
}

thread_local! {
    static DEBUG: Cell<Option<&'static Fiber>> = const { Cell::new(None) };
    static CURRENT: Cell<Option<Rc<Fiber>>> = const { Cell::new(None) };
    static HELD: Cell<Option<Loud>> = const { Cell::new(None) };
    static SHARED: Cell<Option<Rc<Loud>>> = const { Cell::new(None) };
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct RestoreDebug(Option<&'static Fiber>);

impl Drop for RestoreDebug {
    fn drop(&mut self) {
        DEBUG.set(self.0);
    }
}

struct RestoreCurrent(Option<Rc<Fiber>>);

impl Drop for RestoreCurrent {
    fn drop(&mut self) {
        CURRENT.set(self.0.take());
    }
}

struct RestoreHeld(Option<Loud>);

impl Drop for RestoreHeld {
    fn drop(&mut self) {
        HELD.set(self.0.take());
    }
}

struct RestoreShared(Option<Rc<Loud>>);

impl Drop for RestoreShared {
    fn drop(&mut self) {
        SHARED.set(self.0.take());
    }
}

struct RestoreDepth(u32);

impl Drop for RestoreDepth {
    fn drop(&mut self) {
        let was = DEPTH.replace(self.0);
        println!("depth was {was}");
    }
}

struct Swap<'a> {
    cell: &'a Cell<Option<&'static Fiber>>,
    previous: Option<&'static Fiber>,
}

impl Drop for Swap<'_> {
    fn drop(&mut self) {
        self.cell.set(self.previous.take());
    }
}

fn work(n: u32) -> u32 {
    if n > 100 {
        panic!("too big: {n}");
    }
    n + 1
}

fn debug_id() -> u32 {
    DEBUG.with(|debug| debug.get().map_or(0, |fiber| fiber.0))
}

fn current_id() -> u32 {
    CURRENT.with(|current| {
        let fiber = current.take();
        let id = fiber.as_ref().map_or(0, |fiber| fiber.0);
        current.set(fiber);
        id
    })
}

fn with_debug(fiber: &'static Fiber, n: u32) -> u32 {
    let _restore = RestoreDebug(DEBUG.replace(Some(fiber)));
    work(n) + debug_id()
}

fn with_current(fiber: Rc<Fiber>, n: u32) -> u32 {
    let _restore = RestoreCurrent(CURRENT.replace(Some(fiber)));
    work(n) + current_id()
}

fn with_held(n: u32) -> u32 {
    let _restore = RestoreHeld(HELD.replace(Some(Loud(n))));
    work(n)
}

fn with_shared(n: u32) -> u32 {
    let _restore = RestoreShared(SHARED.replace(Some(Rc::new(Loud(n)))));
    work(n)
}

fn deeper(n: u32) -> u32 {
    let _restore = RestoreDepth(DEPTH.replace(n));
    work(n)
}

fn swapped(cell: &Cell<Option<&'static Fiber>>, fiber: &'static Fiber, n: u32) -> u32 {
    let _swap = Swap {
        cell,
        previous: cell.replace(Some(fiber)),
    };
    work(n) + cell.get().map_or(0, |fiber| fiber.0)
}

fn main() {
    static ONE: Fiber = Fiber(10);
    static TWO: Fiber = Fiber(20);
    println!("debug {} then {}", with_debug(&ONE, 1), debug_id());
    let r = panic::catch_unwind(|| with_debug(&TWO, 200));
    println!("debug caught {} then {}", r.is_err(), debug_id());

    println!("current {} then {}", with_current(Rc::new(Fiber(30)), 2), current_id());
    let r = panic::catch_unwind(|| with_current(Rc::new(Fiber(40)), 300));
    println!("current caught {} then {}", r.is_err(), current_id());

    println!("held {}", with_held(3));
    println!("held {}", with_held(4));
    let r = panic::catch_unwind(|| with_held(400));
    println!("held caught {}", r.is_err());

    println!("shared {}", with_shared(5));
    let r = panic::catch_unwind(|| with_shared(500));
    println!("shared caught {}", r.is_err());

    println!("deeper {}", deeper(7));
    let r = panic::catch_unwind(|| deeper(700));
    println!("deeper caught {}", r.is_err());

    let cell = Cell::new(Some(&ONE));
    println!("swapped {} then {}", swapped(&cell, &TWO, 6), cell.get().map_or(0, |f| f.0));
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| swapped(&cell, &TWO, 600)));
    println!("swapped caught {} then {}", r.is_err(), cell.get().map_or(0, |f| f.0));
}

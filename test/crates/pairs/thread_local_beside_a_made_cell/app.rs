//! A thread-local of a cell the library makes too, which std's
//! `thread_local!` points to, is nothing this crate changes in place: the
//! library's `create_fiber` is its own, called, not compiled here, where
//! it couldn't count in the library's `NEXT_ID`.

use std::cell::Cell;

thread_local! {
    static CURRENT: Cell<Option<&'static str>> = const { Cell::new(None) };
}

pub fn main() {
    let fiber = dep::create_fiber();
    fiber.name.set(Some("App"));
    CURRENT.set(fiber.name.get());
    let next = dep::create_fiber();
    println!("{} {} {:?} {:?}", fiber.id, next.id, CURRENT.get(), next.name.get());
}

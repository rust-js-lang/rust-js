// A `pub use` of a type alone has no JS, so it's no import in the
// load-order graph: `stack` re-exports `hidden`'s `Lanes`, and `hidden`'s
// `pub` thread-local, made as its module loads, calls `stack`'s
// `create_cursor`, with no cycle between them, as React's
// ReactFiberHiddenContext makes its cursors of ReactFiberStack's
// (rd116, #295).
pub mod stack {
    use std::cell::Cell;

    pub use super::hidden::Lanes;

    pub struct Cursor {
        pub current: Cell<u32>,
    }

    pub fn create_cursor(value: u32) -> Cursor {
        Cursor {
            current: Cell::new(value),
        }
    }

    pub fn push(cursor: &Cursor, value: Lanes) {
        cursor.current.set(value.0);
    }
}

pub mod hidden {
    use super::stack::{Cursor, create_cursor, push};

    pub struct Lanes(pub u32);

    thread_local! {
        pub static HIDDEN_CURSOR: Cursor = create_cursor(1);
    }

    pub fn hide(lanes: u32) -> u32 {
        HIDDEN_CURSOR.with(|cursor| {
            push(cursor, Lanes(lanes));
            cursor.current.get()
        })
    }
}

fn main() {
    println!("{}", hidden::HIDDEN_CURSOR.with(|c| c.current.get()));
    println!("{}", hidden::hide(4));
}

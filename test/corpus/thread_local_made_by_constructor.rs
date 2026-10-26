// A thread-local whose initializer calls a function of the crate's that
// only makes a value of what it's given, as React's `createCursor` does,
// is made as its module loads, as JS writes it: what it runs can't tell
// when it does (ADR 0037, amended). One whose maker reads another
// thread-local or a static, or prints, is made where it's first used, and
// so is one whose maker is of a module that imports its own back, which
// may load after it, before its `const` is made.

use std::cell::Cell;

mod stack {
    use std::cell::RefCell;

    pub struct Cursor<T> {
        pub current: RefCell<T>,
    }

    pub fn create_cursor<T>(default_value: T) -> Cursor<T> {
        Cursor {
            current: RefCell::new(default_value),
        }
    }

    /// A maker of what another makes.
    pub fn cursor_of_some(value: u32) -> Cursor<Option<u32>> {
        create_cursor(Some(value))
    }
}

static BASE: u32 = 5;

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

fn based() -> stack::Cursor<u32> {
    stack::create_cursor(BASE + 1)
}

fn counted() -> stack::Cursor<u32> {
    COUNT.with(|count| count.set(count.get() + 1));
    stack::create_cursor(COUNT.with(Cell::get))
}

fn noisy() -> stack::Cursor<u32> {
    println!("made noisy");
    stack::create_cursor(7)
}

/// What a `const fn` of std's runs of what it's given, a closure that
/// prints, runs the crate's code too.
fn mapped() -> stack::Cursor<Option<u32>> {
    stack::create_cursor(Some(2).map(|n| {
        println!("mapped {n}");
        n * 2
    }))
}

thread_local! {
    static CURSOR: stack::Cursor<u32> = stack::create_cursor(0);
    static NESTED: stack::Cursor<Option<u32>> = stack::cursor_of_some(1);
    static NONE: stack::Cursor<Option<u32>> = stack::create_cursor(None);
    static BASED: stack::Cursor<u32> = based();
    static COUNTED: stack::Cursor<u32> = counted();
    static NOISY: stack::Cursor<u32> = noisy();
    static MAPPED: stack::Cursor<Option<u32>> = mapped();
}

// `marks` and `uses` import each other, and this module imports `marks`
// first, which loads `uses` before it: `uses`' thread-local, whose maker
// reads `marks`' `const`, is made where it's first used, once both have.
mod marks {
    pub const LIMIT: u32 = 4;

    pub fn limited() -> super::stack::Cursor<u32> {
        super::stack::create_cursor(LIMIT)
    }

    pub fn used() -> u32 {
        super::uses::read()
    }
}

mod uses {
    thread_local! {
        static LIMITED: super::stack::Cursor<u32> = super::marks::limited();
    }

    pub fn read() -> u32 {
        LIMITED.with(|cursor| *cursor.current.borrow())
    }
}

fn read(cursor: &'static std::thread::LocalKey<stack::Cursor<u32>>) -> u32 {
    cursor.with(|cursor| *cursor.current.borrow())
}

fn main() {
    println!("main");
    CURSOR.with(|cursor| cursor.current.replace(3));
    println!("{} {}", read(&CURSOR), read(&BASED));
    NESTED.with(|nested| println!("{:?}", nested.current.borrow()));
    NONE.with(|none| println!("{:?}", none.current.borrow()));
    println!("{} {}", read(&COUNTED), read(&NOISY));
    MAPPED.with(|mapped| println!("{:?}", mapped.current.borrow()));
    println!("{} {}", marks::used(), marks::LIMIT);
}

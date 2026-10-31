// A guard's scope whose value is an `if`'s, React's flushSync's `if let
// Some(f) = f { f() } else { None }`: each branch returns its value in the
// `try`, which runs the guard's `finally` once it's made, as Rust drops the
// guard after its value is made (rd286).

use std::cell::Cell;

thread_local! {
    static PRIORITY: Cell<u32> = const { Cell::new(0) };
}

struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        PRIORITY.set(self.0);
    }
}

fn flush_sync(f: Option<&dyn Fn() -> Option<u32>>) -> Option<u32> {
    let _restore = Restore(PRIORITY.get());
    PRIORITY.set(2);
    if let Some(f) = f { f() } else { None }
}

fn main() {
    println!("{:?}", flush_sync(Some(&|| Some(PRIORITY.get() + 1))));
    println!("{:?} {}", flush_sync(None), PRIORITY.get());
}

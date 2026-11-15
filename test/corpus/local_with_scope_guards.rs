// A thread-local's with/try_with runs only its callback after an eager
// initializer. Nonthrowing restores are a finally; a throwing callback or
// a lazy initializer still keeps Rust's abort-safe cleanup (#1833).
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static VALUE: Cell<u32> = const { Cell::new(0) };
    static LAZY: Cell<u32> = {
        println!("lazy initialized");
        Cell::new(1)
    };
}

struct Restore(u32);
impl Drop for Restore {
    fn drop(&mut self) {
        VALUE.with(|value| value.set(self.0));
    }
}

fn scoped<T>(next: u32, run: impl FnOnce() -> T) -> T {
    let outer = VALUE.with(|value| value.replace(next));
    let _restore = Restore(outer);
    run()
}

struct RestoreWith<F: Fn()>(F);
impl<F: Fn()> Drop for RestoreWith<F> {
    fn drop(&mut self) {
        VALUE.with(|_| (self.0)());
    }
}

fn generic_scope<T>(next: u32, run: impl FnOnce() -> T) -> T {
    let outer = VALUE.with(|value| value.replace(next));
    let _restore = RestoreWith(|| VALUE.set(outer));
    run()
}

struct RestoreTry(u32);
impl Drop for RestoreTry {
    fn drop(&mut self) {
        let _ = VALUE.try_with(|value| value.set(self.0));
    }
}

fn try_scope<T>(next: u32, run: impl FnOnce() -> T) -> T {
    let outer = VALUE.with(|value| value.replace(next));
    let _restore = RestoreTry(outer);
    run()
}

struct LazyRestore;
impl Drop for LazyRestore {
    fn drop(&mut self) {
        LAZY.with(|value| value.set(0));
    }
}

struct Fallible;
impl Drop for Fallible {
    fn drop(&mut self) {
        VALUE.with(|value| {
            if value.get() == 9 {
                panic!("restore failed");
            }
            value.set(0);
        });
    }
}

fn read() -> u32 {
    VALUE.with(Cell::get)
}

fn main() {
    let result = scoped(3, || {
        println!("outer {}", read());
        let nested = scoped(7, || {
            println!("inner {}", read());
            42
        });
        println!("restored {}", read());
        nested
    });
    println!("result {result} after {}", read());
    for generic in [false, true] {
        let mut callback = || {
            println!("throwing {}", read());
            panic!("callback failed");
        };
        let failed = catch_unwind(AssertUnwindSafe(|| {
            if generic {
                generic_scope(5, &mut callback)
            } else {
                scoped(4, &mut callback)
            }
        }));
        println!("caught {} after {}", failed.is_err(), read());
    }
    let result = generic_scope(6, || read() + 10);
    println!("generic {result} after {}", read());
    let result = try_scope(8, || read() + 10);
    println!("try {result} after {}", read());
    {
        let _restore = LazyRestore;
        println!("before lazy restore");
    }
    println!("lazy after {}", LAZY.with(Cell::get));
    VALUE.set(9);
    let failed = catch_unwind(|| {
        let _restore = Fallible;
        println!("before fallible restore");
    });
    println!("restore caught {} value {}", failed.is_err(), read());
}

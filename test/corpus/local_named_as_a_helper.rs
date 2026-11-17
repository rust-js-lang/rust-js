// A local the program names as one of the runtime's helpers is spelled,
// `rust_js::name = "$some"`, is the program's own, never the helper (#2016):
// its calls run, where the helper's would be dropped as doing nothing, its
// panic unwinds through a guard, where the helper's never throws, and beside
// the helper itself, in one function, each is its own.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn counted() -> u32 {
    let n = Cell::new(0);
    #[cfg_attr(rust_js, rust_js::name = "$some")]
    let bump = |x: u32| {
        n.set(n.get() + x);
        x
    };
    bump(2);
    let _ = bump(3);
    n.get()
}

fn guarded() -> bool {
    #[cfg_attr(rust_js, rust_js::name = "$some")]
    let fail = |x: u32| -> u32 {
        if x > 1 {
            panic!("too big");
        }
        x
    };
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let _guard = Noisy("guard");
        fail(2);
    }));
    caught.is_err()
}

fn beside<T: Copy>(value: T) -> (Option<T>, u32) {
    #[cfg_attr(rust_js, rust_js::name = "$some")]
    let twice = |x: u32| x * 2;
    (Some(value), twice(4))
}

fn main() {
    println!("{}", counted());
    println!("{}", guarded());
    let (a, b) = beside(None::<u8>);
    println!("{a:?} {b}");
    let (c, d) = beside(7u8);
    println!("{c:?} {d}");
}

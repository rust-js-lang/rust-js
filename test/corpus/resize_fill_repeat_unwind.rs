// `resize`, `fill` and `vec![x; n]` own the value they clone from till the
// last: it's dropped as a clone of it, or a drop of what they replace or
// cut off, panics; the clones `resize` pushed stay the `Vec`'s, and those
// `vec!` made are dropped after it, as std's `from_elem` drops its `Vec`
// (#898).
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        if self.0 == 9 && COUNT.get() == 2 {
            panic!("clone");
        }
        COUNT.set(COUNT.get() + 1);
        D(self.0)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 5 {
            panic!("drop");
        }
    }
}

thread_local! {
    static COUNT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    COUNT.set(0);
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("resize longer", || {
        let mut v = vec![D(1)];
        v.resize(5, D(9));
        println!("{}", v.len());
    });
    run("resize shorter", || {
        let mut v = vec![D(1), D(5), D(2)];
        v.resize(1, D(7));
        println!("{}", v.len());
    });
    run("fill", || {
        let mut v = vec![D(1), D(2), D(3), D(4)];
        v.fill(D(9));
        println!("{}", v.len());
    });
    run("fill's drop", || {
        let mut v = vec![D(1), D(5), D(3)];
        v.fill(D(8));
        println!("{}", v.len());
    });
    run("vec!", || {
        let v = vec![D(9); 4];
        println!("{}", v.len());
    });
    run("deque resize", || {
        let mut q: VecDeque<D> = VecDeque::new();
        q.push_back(D(1));
        q.resize(5, D(9));
        println!("{}", q.len());
    });
}

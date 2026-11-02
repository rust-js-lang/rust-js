// A `catch_unwind` whose closure's value nothing reads, only whether it
// panicked, `catch_unwind(|| check(n)).is_err()`: the value is still
// evaluated in the `try`, as Rust evaluates it, where it may panic (#462):
// a call, an index past the end, a division by zero, an `unwrap` of
// `None`, a cell borrowed twice and text cut inside a `char`. A JS
// operator of any value, `js::has(m, "default")`'s `"default" in m`, which
// native Rust can't run, is test/lowering-unwind-arms.test.ts's.
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn check(n: u32) -> u32 {
    if n > 2 {
        panic!("big {n}");
    }
    n + 1
}

fn called(n: u32) -> bool {
    catch_unwind(|| check(n)).is_err()
}

fn indexed(items: &[u32], i: usize) -> bool {
    catch_unwind(|| items[i]).is_err()
}

fn divided(a: u32, b: u32) -> bool {
    catch_unwind(|| a / b).is_err()
}

fn unwrapped(found: Option<u32>) -> bool {
    catch_unwind(|| found.unwrap()).is_ok()
}

fn borrowed(cell: &RefCell<u32>, held: bool) -> bool {
    let _guard = held.then(|| cell.borrow());
    catch_unwind(AssertUnwindSafe(|| *cell.borrow_mut())).is_err()
}

fn cut(text: &str, at: usize) -> bool {
    catch_unwind(|| text[..at].len()).is_err()
}

fn main() {
    let items = [1, 2, 3];
    for n in [1, 3] {
        println!("called {n} {}", called(n));
    }
    for i in [2, 3] {
        println!("indexed {i} {}", indexed(&items, i));
    }
    for b in [0, 2] {
        println!("divided {b} {}", divided(6, b));
    }
    println!("unwrapped {} {}", unwrapped(Some(4)), unwrapped(None));
    let cell = RefCell::new(5);
    println!("borrowed {} {}", borrowed(&cell, false), borrowed(&cell, true));
    for at in [1, 2] {
        println!("cut {at} {}", cut("é!", at));
    }
}

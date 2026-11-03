// A `for` over a slice in an `if` in a drop guard's scope is the `for` it
// is out of one, an index loop, from the back too, in the `try` whose
// `finally` drops the guard: not `const iter = $iter(xs.toReversed());
// while ((x = $next(iter)) != null)`, as each step's own `try` held it apart
// from what makes a `for` (rd711). As React's ReactChildFiber finds the
// thrown fiber's owner, its debug info read from the back, in the closure
// that restores its debug info as it's left.
use std::cell::Cell;

thread_local! {
    static CURRENT: Cell<u32> = const { Cell::new(0) };
}

struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restore {}", self.0);
        CURRENT.set(self.0);
    }
}

fn owner(xs: Option<&[u32]>, out: &Cell<u32>) -> u32 {
    let _restore = Restore(CURRENT.get());
    CURRENT.set(9);
    if let Some(xs) = xs {
        for &x in xs.iter().rev() {
            if x > 2 {
                out.set(x);
                break;
            }
        }
    }
    println!("current {}", CURRENT.get());
    out.get()
}

fn first(xs: Option<&[u32]>, out: &Cell<u32>) -> u32 {
    let _restore = Restore(CURRENT.get());
    if let Some(xs) = xs {
        for &x in xs {
            out.set(out.get() + x);
        }
    }
    out.get()
}

fn caught(xs: Option<&'static [u32]>) -> Box<dyn Fn(fn() -> u32) -> u32> {
    Box::new(move |run: fn() -> u32| {
        let _restore = Restore(CURRENT.get());
        match std::panic::catch_unwind(run) {
            Ok(n) => n,
            Err(_) => {
                let mut found = 0;
                if let Some(xs) = xs {
                    for &x in xs.iter().rev() {
                        if x % 2 == 0 {
                            found = x;
                            break;
                        }
                    }
                }
                found
            }
        }
    })
}

fn fails() -> u32 {
    panic!("thrown")
}

fn main() {
    let out = Cell::new(0);
    println!("{}", owner(Some(&[1, 5, 2, 4, 1]), &out));
    println!("{}", owner(None, &out));
    println!("{}", first(Some(&[1, 2, 3]), &out));
    let f = caught(Some(&[2, 3, 4, 5]));
    println!("{}", f(|| 7));
    println!("{}", f(fails));
}

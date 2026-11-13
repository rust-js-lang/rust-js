// A `collect()` into an `Rc<[T]>` or an `Arc<[T]>` of a `TrustedLen` whose
// steps may panic, of items that drop: std's `from_iter_exact` writes each
// item in place, and a panic as it steps drops what's left of the iterator
// before what it wrote, where a `Vec`'s collect, and an iterator that isn't
// `TrustedLen`, drop what was written first (#1678).
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

thread_local! {
    static STEPS: Cell<u32> = const { Cell::new(0) };
    static PANIC_AT: Cell<u32> = const { Cell::new(0) };
}

fn step() {
    STEPS.with(|n| {
        n.set(n.get() + 1);
        if n.get() == PANIC_AT.with(Cell::get) {
            panic!("step {}", n.get());
        }
    });
}

// Collected by generic code, whose caller says it's `TrustedLen`.
fn made<C: FromIterator<Loud>, I: Iterator<Item = Loud>>(items: I) -> C {
    items.collect()
}

fn louds() -> Vec<Loud> {
    vec![Loud(1), Loud(2), Loud(3), Loud(4)]
}

fn main() {
    for panic_at in [0, 1, 3, 4] {
        PANIC_AT.with(|p| p.set(panic_at));
        for which in 0..9 {
            STEPS.with(|n| n.set(0));
            println!("panic at {panic_at}, case {which}");
            let made = catch_unwind(AssertUnwindSafe(|| {
                let mapped = louds().into_iter().map(|x| {
                    step();
                    x
                });
                match which {
                    0 => mapped.collect::<Rc<[Loud]>>().len(),
                    1 => mapped.collect::<Arc<[Loud]>>().len(),
                    2 => mapped.rev().collect::<Rc<[Loud]>>().len(),
                    3 => mapped.filter(|_| true).collect::<Rc<[Loud]>>().len(),
                    4 => mapped.collect::<Vec<Loud>>().len(),
                    5 => {
                        let tags = [10u8, 20, 30, 40];
                        mapped
                            .zip(tags)
                            .map(|(x, t)| Loud(x.0 + t))
                            .collect::<Rc<[Loud]>>()
                            .len()
                    }
                    6 => made::<Rc<[Loud]>, _>(mapped).len(),
                    7 => made::<Arc<[Loud]>, _>(mapped.filter(|_| true)).len(),
                    _ => (0..4u8)
                        .map(|i| {
                            step();
                            Loud(i + 5)
                        })
                        .collect::<Arc<[Loud]>>()
                        .len(),
                }
            }));
            println!("made {:?}", made.ok());
        }
    }
}

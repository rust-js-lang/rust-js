// A closure that holds a value with a destructor, made as a block's value
// given straight to a call (#759): the block's own locals dropped as the
// block ends, before the call, and what the closure took dropped with it,
// where the call drops it, as a panic in it unwinds too.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn apply(f: impl FnOnce() -> u32) -> u32 {
    println!("apply");
    let n = f();
    println!("applied");
    n
}

fn twice(f: impl Fn(u32) -> u32) -> u32 {
    f(1) + f(2)
}

fn main() {
    let mut seen = 0;
    let n = vec![1u32, 2, 3].into_iter().fold(0, {
        let g = D(90);
        let seen = &mut seen;
        move |a, x| {
            let _g = &g;
            *seen += 1;
            a + x
        }
    });
    println!("{n} after {seen}");

    let v: Vec<u32> = vec![1u32, 2]
        .into_iter()
        .map({
            let h = D(80);
            move |x| {
                let _h = &h;
                x + 10
            }
        })
        .collect();
    println!("{v:?}");

    // Its block's own locals drop as the block ends, before the call.
    let n = apply({
        let local = D(1);
        let g = D(2);
        println!("made with {}", local.0);
        move || {
            let _g = &g;
            g.0
        }
    });
    println!("apply {n}");

    let n = twice({
        let g = D(3);
        move |x| {
            let _g = &g;
            x + g.0
        }
    });
    println!("twice {n}");

    let mut seen = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![1u32, 2, 3].into_iter().for_each({
            let g = D(91);
            let seen = &mut seen;
            move |x| {
                let _g = &g;
                *seen += 1;
                if *seen == 2 {
                    panic!("consumer");
                }
                println!("item {x}");
            }
        })
    }));
    println!("panicked {} after {seen}", r.is_err());

    // A block's value that's a block's, for a `let` and given to a call.
    let f = {
        let a = D(4);
        {
            let g = D(5);
            println!("inner {}", a.0);
            move || {
                let _g = &g;
                g.0
            }
        }
    };
    println!("f {}", f());
    drop(f);
    let n: u32 = [1u32, 2]
        .iter()
        .map({
            let a = D(6);
            {
                let g = D(7);
                println!("inner {}", a.0);
                move |x| {
                    let _g = &g;
                    x + g.0
                }
            }
        })
        .sum();
    println!("sum {n}");

    // Through what passes it on, a block given to `catch_unwind` as an
    // `AssertUnwindSafe`, its closure panicking.
    let r = catch_unwind(AssertUnwindSafe({
        let g = D(8);
        move || {
            let _g = &g;
            panic!("in {}", g.0);
        }
    }));
    println!("caught {}", r.is_err());
    println!("end");
}

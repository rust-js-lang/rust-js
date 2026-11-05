// A `skip_while()` whose predicate panics in its first step, inside a
// `fold` or a `try_fold` by a closure that holds a destructor, of an
// accumulator with one: std's `SkipWhile::fold(self, init, fold)` and
// `try_fold(&mut self, init, fold)` hold nothing but their parameters
// then, so `fold` drops first, then `init`, then what's left (#768); and
// so do a `skip()`'s `nth(n - 1)` and a `step_by()`'s first `next`. Past
// that step, and through any other adapter, std's own fold holds the
// accumulator in its body, which drops first.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's own iterator, whose first `next` panics.
struct Own(u32);

impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.0 += 1;
        assert!(self.0 != 1, "own");
        (self.0 < 5).then_some(D("own"))
    }
}

fn skipping(n: u32, calls: &mut u32) -> bool {
    *calls += 1;
    assert!(*calls != 2, "skip_while");
    n < 2
}

fn main() {
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let mut calls = 0;
        vec![1u32, 2, 3]
            .into_iter()
            .skip_while(|&x| skipping(x, &mut calls))
            .fold(D("acc"), move |a, _| {
                let _g = &g;
                a
            });
    }));
    println!("fold panicked {}", r.is_err());

    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let mut calls = 0;
        let _ = (1..4u32)
            .skip_while(|&x| skipping(x, &mut calls))
            .try_fold(D("acc"), move |a, _| {
                let _g = &g;
                Some(a)
            });
    }));
    println!("try_fold panicked {}", r.is_err());

    // Of items with a destructor, what's left of the `Vec` after both.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let mut calls = 0;
        vec![D("1"), D("2"), D("3")]
            .into_iter()
            .skip_while(|x| skipping(x.0.len() as u32, &mut calls))
            .fold(D("acc"), move |a, _| {
                let _g = &g;
                a
            });
    }));
    println!("fold of drops panicked {}", r.is_err());

    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let mut it = vec![D("1"), D("2"), D("3")].into_iter();
        let mut calls = 0;
        let _: Result<D, u32> = it
            .by_ref()
            .skip_while(|x| skipping(x.0.len() as u32, &mut calls))
            .try_fold(D("acc"), move |a, _| {
                let _g = &g;
                Ok(a)
            });
    }));
    println!("try_fold of drops panicked {}", r.is_err());

    // A `skip()`'s `nth(n - 1)` is such a first step too, of `fold` and
    // `try_fold`.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        vec![D("1"), D("2"), D("3"), D("4")]
            .into_iter()
            .map(|x| {
                assert!(x.0 != "2", "skip");
                x
            })
            .skip(3)
            .fold(D("acc"), move |a, _| {
                let _g = &g;
                a
            });
    }));
    println!("skip fold panicked {}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let _ = vec![D("1"), D("2"), D("3"), D("4")]
            .into_iter()
            .map(|x| {
                assert!(x.0 != "2", "skip");
                x
            })
            .skip(3)
            .try_fold(D("acc"), move |a, _| {
                let _g = &g;
                Some(a)
            });
    }));
    println!("skip try_fold panicked {}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        Own(0).step_by(2).fold(D("acc"), move |a, _| {
            let _g = &g;
            a
        });
    }));
    println!("step_by fold panicked {}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let _ = Own(0).step_by(2).try_fold(D("acc"), move |a, _| {
            let _g = &g;
            Some(a)
        });
    }));
    println!("step_by try_fold panicked {}", r.is_err());
    // Of a `map()` whose function may panic, which the build that rewrites
    // adapters would make in place as a filter by index (#773).
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        vec![D("1"), D("2"), D("3")]
            .into_iter()
            .map(|x| {
                assert!(x.0 != "1", "first");
                x
            })
            .step_by(2)
            .fold(D("acc"), move |a, _| {
                let _g = &g;
                a
            });
    }));
    println!("mapped step_by fold panicked {}", r.is_err());

    // A panic of a `map()` past it, inside the source's fold: the
    // accumulator first.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let _ = (1..4u32)
            .map(|x| {
                assert!(x != 2, "map");
                x
            })
            .try_fold(D("acc"), move |a, _| {
                let _g = &g;
                Some(a)
            });
    }));
    println!("map try_fold panicked {}", r.is_err());

    // Through a `skip()` of none, which gives its source's `try_fold` what
    // drops with `f`, as std's moves `fold` into it.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D("f");
        let mut calls = 0;
        let _ = (1..4u32)
            .skip_while(|&x| skipping(x, &mut calls))
            .skip(0)
            .try_fold(D("acc"), move |a, _| {
                let _g = &g;
                Some(a)
            });
    }));
    println!("skip of none try_fold panicked {}", r.is_err());

    // No panic, of a predicate that may: what it folds into given back, the
    // closure dropped at the end.
    let g = D("f");
    let mut calls = 10;
    let acc = vec![1u32, 2, 3]
        .into_iter()
        .skip_while(|&x| skipping(x, &mut calls))
        .try_fold(D("acc"), move |a, x| {
            let _g = &g;
            println!("item {x}");
            Some(a)
        });
    println!("folded {}", acc.map_or("none", |a| a.0));
}

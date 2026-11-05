// A closure a std call is given by value and calls by `&mut` or `&`,
// whose call panics: std's call drops it, and what it captured, as the
// panic unwinds (#719), a `map`'s that a fold moved in (#710), a
// `for_each`'s, a `fold`'s, an `any`'s, a `position`'s, a `retain`'s and a
// `sort_by_key`'s, as the crate builds it and with std's iterator structs.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("d{} ", self.0);
    }
}

// What reads the guard a closure holds, so it's moved in whole.
fn zero(_guard: &D) -> u32 {
    0
}

fn case(name: &str, run: impl FnOnce(D)) {
    print!("{name}: ");
    let r = catch_unwind(AssertUnwindSafe(|| run(D(9))));
    println!("{}", if r.is_err() { "panicked" } else { "" });
}

fn main() {
    case("a map's, folded", |g| {
        let n: u32 = (1..6)
            .map(D)
            .collect::<Vec<_>>()
            .into_iter()
            .take(3)
            .map(move |d| {
                assert!(d.0 != 2, "two");
                d.0 + zero(&g)
            })
            .sum();
        print!("{n} ");
    });
    case("for_each", |g| {
        vec![1u32, 2, 3]
            .into_iter()
            .for_each(move |x| assert!(x + zero(&g) != 2));
    });
    case("fold", |g| {
        let n = vec![1u32, 2, 3].into_iter().fold(0, move |a, x| {
            assert!(x != 2);
            a + x + zero(&g)
        });
        print!("{n} ");
    });
    case("any", |g| {
        let n = vec![1u32, 2, 3].into_iter().any(move |x| {
            assert!(x != 2);
            x + zero(&g) == 9
        });
        print!("{n} ");
    });
    case("retain", |g| {
        let mut v = vec![D(1), D(2), D(3)];
        v.retain(move |d| {
            assert!(d.0 != 2);
            d.0 + zero(&g) > 0
        });
        print!("{} ", v.len());
    });
    case("sort_by_key", |g| {
        let mut v = vec![3u32, 1, 2];
        v.sort_by_key(move |&x| {
            assert!(x != 2);
            x + zero(&g)
        });
        print!("{v:?} ");
    });
    case("position", |g| {
        let p = [1u32, 2, 3].iter().position(move |&x| {
            assert!(x != 2);
            x + zero(&g) == 9
        });
        print!("{p:?} ");
    });
    case("ok no panic", |g| {
        let n: u32 = vec![1u32, 2].into_iter().fold(0, move |a, x| a + x + zero(&g));
        print!("{n} ");
    });
}

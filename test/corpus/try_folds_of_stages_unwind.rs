// Folds whose stage panics, a closure an `impl FnMut` is, under
// `catch_unwind`: the accumulator it strands is dropped as std unwinds, of
// an `enumerate()`'s `rfold` and `try_fold` (rd767), of a `try_fold` into
// an `Option` or a `Result`, a chain's too, and of a fold of a chain held,
// whose stage only its type shows (rd767).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds(r: std::ops::Range<u32>) -> Vec<D> {
    r.map(D).collect()
}

fn boom(at: u32) -> impl FnMut(D) -> D {
    move |d| {
        if d.0 == at {
            panic!("boom");
        }
        d
    }
}

fn push(mut acc: Vec<D>, d: D) -> Vec<D> {
    acc.push(d);
    acc
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("enumerate rfold", || {
        ds(40..44)
            .into_iter()
            .map(boom(41))
            .enumerate()
            .rfold(Vec::new(), |acc, (_, d)| push(acc, d))
            .len()
    });
    run("chain try_fold", || {
        ds(60..62)
            .into_iter()
            .chain(ds(62..65).into_iter().map(boom(63)))
            .try_fold(Vec::new(), |acc, d| Some(push(acc, d)))
            .map_or(0, |v| v.len())
    });
    run("map try_fold result", || {
        let r: Result<Vec<D>, ()> = ds(70..74)
            .into_iter()
            .map(boom(72))
            .try_fold(Vec::new(), |acc, d| Ok(push(acc, d)));
        r.map_or(0, |v| v.len())
    });
    run("enumerate try_fold", || {
        ds(80..84)
            .into_iter()
            .map(boom(82))
            .enumerate()
            .try_fold(Vec::new(), |acc, (_, d)| Some(push(acc, d)))
            .map_or(0, |v| v.len())
    });
    run("chain fold", || {
        ds(90..92)
            .into_iter()
            .chain(ds(92..95).into_iter().map(boom(93)))
            .fold(Vec::new(), push)
            .len()
    });
}

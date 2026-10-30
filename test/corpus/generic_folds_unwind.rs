// A generic `rfold` and `try_fold`, and a `try_fold` and `rfold` of an
// owning iterator, whose stage panics part-way: the stage's item dropped,
// then the accumulator the fold holds, then what's left, as std's frames
// unwind (#423). And of a `map` stepped from its back by `rev()`, held or
// not, by a `fold` and by a `take_while`'s, a `try_fold` of its
// `try_rfold`, and by its own `rfold` and `try_rfold`, which pass the
// accumulator's drop on to the map's.
use std::ops::ControlFlow;
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

fn boom(d: &D, at: u32) {
    if d.0 == at {
        panic!("boom");
    }
}

fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn backwards<I: DoubleEndedIterator<Item = D>>(items: I) -> Vec<D> {
    items.rfold(Vec::new(), |mut acc, d| {
        acc.push(d);
        acc
    })
}

fn tried_back<I: DoubleEndedIterator<Item = D>>(mut items: I) -> ControlFlow<Vec<D>, Vec<D>> {
    items.try_rfold(Vec::new(), |mut acc, d| {
        acc.push(d);
        ControlFlow::Continue(acc)
    })
}

// What steps from its back by an `rfold` of its own, which a `map` of it
// passes on.
struct Count(u32, u32);

impl Iterator for Count {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0 - 1)
        })
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1)
        })
    }
    fn rfold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

fn tried<I: Iterator<Item = D>>(mut items: I) -> ControlFlow<Vec<D>, Vec<D>> {
    items.try_fold(Vec::new(), |mut acc, d| {
        acc.push(d);
        ControlFlow::Continue(acc)
    })
}

fn push(mut acc: Vec<D>, d: D) -> Vec<D> {
    acc.push(d);
    acc
}

fn main() {
    run("generic rfold", || {
        let x = backwards(ds(0..4).into_iter().inspect(|d| boom(d, 1)));
        println!("{}", x.len());
    });
    run("generic try_fold", || {
        let x = tried(ds(10..14).into_iter().inspect(|d| boom(d, 12)));
        println!("{}", matches!(x, ControlFlow::Continue(_)));
    });
    run("try_fold", || {
        let x = ds(20..24)
            .into_iter()
            .inspect(|d| boom(d, 22))
            .try_fold(Vec::new(), |mut acc, d| {
                acc.push(d);
                ControlFlow::<Vec<D>, Vec<D>>::Continue(acc)
            });
        println!("{}", matches!(x, ControlFlow::Continue(_)));
    });
    run("rfold", || {
        let x = ds(30..34)
            .into_iter()
            .map(|d| {
                boom(&d, 31);
                d
            })
            .rfold(Vec::new(), |mut acc, d| {
                acc.push(d);
                acc
            });
        println!("{}", x.len());
    });
    run("map rev take_while", || {
        let x = ds(40..44)
            .into_iter()
            .map(|d| {
                boom(&d, 41);
                d
            })
            .rev()
            .take_while(|_| true)
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("held map rev", || {
        let held = ds(50..54).into_iter().map(|d| {
            boom(&d, 51);
            d
        });
        let x = held.rev().fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("held map rev take_while", || {
        let held = ds(60..64).into_iter().map(|d| {
            boom(&d, 61);
            d
        });
        let x = held.rev().take_while(|_| true).fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("held map rfold", || {
        let held = ds(70..74).into_iter().map(|d| {
            boom(&d, 71);
            d
        });
        let x = held.rfold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("held map try_rfold", || {
        let mut held = ds(80..84).into_iter().map(|d| {
            boom(&d, 81);
            d
        });
        let x = held.try_rfold(Vec::new(), |mut acc, d| {
            acc.push(d);
            ControlFlow::<Vec<D>, Vec<D>>::Continue(acc)
        });
        println!("{}", matches!(x, ControlFlow::Continue(_)));
    });
    run("generic try_rfold", || {
        let x = tried_back(ds(90..94).into_iter().inspect(|d| boom(d, 92)));
        println!("{}", matches!(x, ControlFlow::Continue(_)));
    });
    run("generic rfold of an own rfold", || {
        let x = backwards(Count(100, 104).map(|d| {
            boom(&d, 102);
            d
        }));
        println!("{}", x.len());
    });
}

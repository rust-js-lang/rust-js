// A stage that panics part-way through a fold of the crate's own iterator,
// which writes its own `rfold`, from its back, or by a `step_by`: the
// stage's item, then the accumulator its adapter holds, then what's left
// dropped, as std's frames unwind, through `map`, `filter`, `enumerate`,
// `inspect` and `step_by` (#423).
use std::panic::{AssertUnwindSafe, catch_unwind};
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Count(u32, u32);
impl Iterator for Count {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0 - 1)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}
impl ExactSizeIterator for Count {}
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
fn boom(d: &D, at: u32) -> bool {
    if d.0 == at {
        panic!("boom");
    }
    true
}
fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}
fn push(mut acc: Vec<D>, d: D) -> Vec<D> {
    acc.push(d);
    acc
}
fn main() {
    run("own map rev", || {
        let x = Count(0, 4)
            .map(|d| {
                boom(&d, 1);
                D(d.0 + 100)
            })
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("own filter rev", || {
        let x = Count(10, 14).filter(|d| boom(d, 11)).rev().fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("own enumerate rev", || {
        let x = Count(20, 24)
            .enumerate()
            .rev()
            .map(|(i, d)| {
                println!("at {i}");
                boom(&d, 21);
                d
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("own step_by", || {
        let x = Count(40, 46)
            .inspect(|d| {
                boom(d, 44);
            })
            .step_by(2)
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("own inspect rev", || {
        let x = Count(30, 34)
            .inspect(|d| {
                boom(d, 31);
            })
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("own stage then enumerate", || {
        let x = Count(50, 54)
            .inspect(|d| {
                boom(d, 52);
            })
            .enumerate()
            .fold(Vec::new(), |mut acc, (i, d)| {
                println!("at {i}");
                acc.push(d);
                acc
            });
        println!("{}", x.len());
    });
}

// A fold of the crate's own iterator, reached through std's adapters,
// `map`, `rev` and `inspect`, whose stage panics part-way: its own `fold`
// and `rfold` take what the runtime gives them, the accumulator's drop, as
// their `B`'s, not as their impl's `T`'s, which comes first, `impl<T>
// Iterator for Wrap<T>`'s; and drop what they took as they unwind, once,
// a generic impl's and another's, `Owned`'s (#423).
use std::panic::{AssertUnwindSafe, catch_unwind};
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Wrap<T> {
    items: Vec<T>,
    extra: Option<T>,
}
impl<T> Iterator for Wrap<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.items.is_empty() {
            self.extra.take()
        } else {
            Some(self.items.remove(0))
        }
    }
    fn fold<B, F: FnMut(B, T) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}
impl<T> DoubleEndedIterator for Wrap<T> {
    fn next_back(&mut self) -> Option<T> {
        self.items.pop()
    }
    fn rfold<B, F: FnMut(B, T) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}
struct Owned {
    items: Vec<D>,
}
impl Iterator for Owned {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.items.pop()
    }
    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("owned fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
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
    run("map fold", || {
        let w = Wrap {
            items: vec![D(1), D(2), D(3)],
            extra: Some(D(9)),
        };
        let x = w
            .map(|d| {
                if d.0 == 2 {
                    panic!("2");
                }
                d
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("map rfold", || {
        let w = Wrap {
            items: vec![D(1), D(2), D(3)],
            extra: Some(D(8)),
        };
        let x = w
            .map(|d| {
                if d.0 == 2 {
                    panic!("2");
                }
                d
            })
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("inspect fold", || {
        let w = Wrap {
            items: vec![D(4), D(5), D(6)],
            extra: None,
        };
        let x = w
            .inspect(|d| {
                if d.0 == 5 {
                    panic!("5");
                }
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("owned map fold", || {
        let owned = Owned {
            items: vec![D(7), D(8), D(9)],
        };
        let x = owned
            .map(|d| {
                if d.0 == 8 {
                    panic!("8");
                }
                d
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
}

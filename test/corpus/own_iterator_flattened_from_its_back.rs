// A `rev()` stepped from its back by its own `rfold`, `nth_back` or
// `rfind`, which MIR holds in a variable first: past a call that may
// unwind and drop it, `String::new()`'s or `mk()`'s, or lent to
// `nth_back` as a `&mut` (#702). Made knowing its back where it's made, a
// flatten's of the crate's iterator, of a `Vec`'s, of the crate's iterators
// themselves, a `flat_map`'s, a map's of a slice's items, a range's and
// the crate's iterator's own, each as std steps it: `rev().rfold` is its
// source's `fold`, `rev().nth_back` its `nth`, `rev().rfind` its `find`;
// a type parameter's as before, by what it's given; std's struct a `Vec`
// holds of the same type, given its back where it's stepped; a flatten
// MIR holds before its `rev()`, past a call whose cleanup drops what's
// live beside it; and as std unwinds where the crate's `next` panics
// part-way.
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl fmt::Display for D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

struct Folds(u32, u32);

impl Iterator for Folds {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        println!("next {}", self.0);
        if self.0 == 7 {
            panic!("no 7");
        }
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0 - 1)
        })
    }
}

impl DoubleEndedIterator for Folds {
    fn next_back(&mut self) -> Option<D> {
        println!("next_back {}", self.1);
        if self.1 == 9 {
            panic!("no 9");
        }
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1)
        })
    }
}

fn mk() -> usize {
    println!("mk");
    1
}

fn shown(acc: String, d: D) -> String {
    acc + &d.to_string()
}

fn backwards<I: DoubleEndedIterator<Item = D>>(it: I) -> String {
    it.rev().rfold(String::new(), shown)
}

fn run(name: &str, f: impl FnOnce() -> String) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("rev rfold", || {
        Folds(0, 2)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rev()
            .rfold(String::new(), shown)
    });
    run("flat_map rev rfold", || {
        Folds(0, 2)
            .flat_map(|d| vec![D(d.0 * 10), d])
            .rev()
            .rfold(String::new(), shown)
    });
    run("vec rev rfold", || {
        vec![D(1), D(2)]
            .into_iter()
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rev()
            .rfold(String::new(), shown)
    });
    run("vec map rev rfold", || {
        vec![D(3), D(4)]
            .into_iter()
            .map(|d| D(d.0 + 1))
            .rev()
            .rfold(String::new(), shown)
    });
    run("rfold", || {
        Folds(0, 2)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rfold(String::new(), shown)
    });
    run("rev fold", || {
        Folds(0, 2)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rev()
            .fold(String::new(), shown)
    });
    run("own flattened rev rfold", || {
        vec![Folds(0, 2), Folds(4, 6)]
            .into_iter()
            .flatten()
            .rev()
            .rfold(String::new(), shown)
    });
    run("own mapped flattened rev rfold", || {
        Folds(0, 2)
            .map(|d| Folds(d.0, d.0 + 2))
            .flatten()
            .rev()
            .rfold(String::new(), shown)
    });
    run("held revs flattened rfold", || {
        vec![Folds(0, 2).rev(), Folds(4, 6).rev()]
            .into_iter()
            .flatten()
            .rfold(String::new(), shown)
    });
    run("rev nth_back", || {
        let d = Folds(0, 2).map(|d| vec![D(d.0 * 10), d]).flatten().rev().nth_back(1);
        format!("{:?}", d.map(|d| d.0))
    });
    run("rev rfind", || {
        let d = Folds(0, 3)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rev()
            .rfind(|d| d.0 == 1);
        format!("{:?}", d.map(|d| d.0))
    });
    run("own rev nth_back", || {
        let d = Folds(0, 3).rev().nth_back(mk());
        format!("{:?}", d.map(|d| d.0))
    });
    run("slice map rev nth_back", || {
        let v = [1u32, 2, 3];
        format!("{:?}", v.iter().map(|x| x * 2).rev().nth_back(1))
    });
    run("range rev nth_back", || format!("{:?}", (0..5u32).rev().nth_back(mk())));
    run("generic rev rfold", || backwards(Folds(0, 2)));
    run("flattens beside what's live", || {
        let found = Folds(0, 2)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rev()
            .rfind(|d| d.0 > 0);
        let last = Folds(0, 2).map(|d| vec![D(d.0 * 10), d]).flatten().next_back();
        let s = Folds(0, 3)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rev()
            .rfold(String::new(), shown);
        let s = vec![Folds(0, 2), Folds(4, 6)]
            .into_iter()
            .flatten()
            .rev()
            .fold(s, shown);
        format!("{:?} {:?} {s}", found.map(|d| d.0), last.map(|d| d.0))
    });
    run("rev rfold panics", || {
        Folds(6, 9)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rev()
            .rfold(String::new(), shown)
    });
    run("rfold panics", || {
        Folds(7, 10)
            .map(|d| vec![D(d.0 * 10), d])
            .flatten()
            .rfold(String::new(), shown)
    });
}

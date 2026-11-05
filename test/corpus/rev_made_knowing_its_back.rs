// A `rev()` stepped from its back that MIR holds in a variable first,
// where no form of it is left to read, made knowing its back where it's
// made (#718): of a type parameter given a JS iterator, `$rev` of it,
// which passes its own `nth`, `fold` and `try_fold` on as std's `Rev` does;
// of a range's map or filter, given a call's value before its `rfold`, what
// steps it from its back, not its numbers reversed inward; and a chain of
// items with a destructor, made knowing its back where it's made.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 13 {
            panic!("drop");
        }
    }
}

/// The crate's iterator, double-ended, with its own `nth`.
struct Count(u32, u32);

impl Iterator for Count {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
    fn nth(&mut self, n: usize) -> Option<D> {
        println!("own nth {n}");
        self.0 += n as u32;
        self.next()
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
}

fn nth_back<I: DoubleEndedIterator<Item = D>>(it: I) -> Option<u32> {
    it.rev().nth_back(1).map(|d| d.0 + 0)
}

fn ends<I: DoubleEndedIterator<Item = D>>(it: I) -> Option<u32> {
    let mut r = it.rev();
    let a = r.next_back().map(|d| d.0 + 0);
    let b = r.next().map(|d| d.0 + 0);
    a.zip(b).map(|(a, b)| a * 100 + b)
}

fn rfold<I: DoubleEndedIterator<Item = D>>(it: I) -> String {
    it.rev().rfold(String::new(), |s, d| format!("{s}{}", d.0))
}

fn rfind<I: DoubleEndedIterator<Item = D>>(it: I) -> Option<u32> {
    it.rev().rfind(|d| d.0 % 2 == 0).map(|d| d.0 + 0)
}

fn numbers<I: DoubleEndedIterator<Item = u32>>(it: I) -> Option<u32> {
    it.rev().nth_back(1)
}

fn mk() -> String {
    String::from("<")
}

fn main() {
    println!("{:?}", nth_back(vec![D(1), D(2), D(3), D(4)].into_iter()));
    println!("{:?}", nth_back(Count(0, 5)));
    println!("{:?}", ends(vec![D(1), D(2), D(3)].into_iter()));
    println!("{:?}", ends(Count(0, 4)));
    println!("{}", rfold(vec![D(1), D(2), D(3)].into_iter()));
    println!("{}", rfold(Count(0, 3)));
    println!("{:?}", rfind(vec![D(1), D(2), D(3), D(5)].into_iter()));
    println!("{:?}", rfind(Count(0, 5)));
    println!("{:?}", numbers(vec![1u32, 2, 3, 4].into_iter()));
    println!("{:?}", numbers(1..5u32));
    let r = catch_unwind(AssertUnwindSafe(|| nth_back(vec![D(11), D(12), D(13), D(14)].into_iter())));
    println!("{}", r.is_err());

    let s = (0..5u32).map(|n| n * 2).rev().rfold(mk(), |s, n| format!("{s}{n}"));
    println!("{s}");
    let s = (0..=4u32).filter(|n| n % 2 == 0).rev().rfold(mk(), |s, n| format!("{s}{n}"));
    println!("{s}");

    // A flatten of a slice's maps, held and stepped from its back.
    let groups = vec![vec![1u32, 2], vec![3, 4]];
    let mut it = groups.iter().map(|g| g.iter().map(|x| x * 10)).flatten();
    println!("{:?} {:?}", it.next_back(), it.next());

    let s = vec![D(1), D(2)].into_iter().chain(vec![D(3)]).rev().fold(0, |s, d| s * 10 + d.0);
    println!("{s}");
    let s = vec![D(1), D(2)].into_iter().chain(vec![D(3)]).rev().rfold(0, |s, d| s * 10 + d.0);
    println!("{s}");
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![D(1), D(2)].into_iter().chain(vec![D(3), D(4)]).rev().rfold(0, |s, d| {
            if d.0 == 2 {
                panic!("fold");
            }
            s * 10 + d.0
        })
    }));
    println!("{}", r.is_err());
}

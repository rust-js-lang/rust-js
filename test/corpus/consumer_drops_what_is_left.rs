// What consumes an iterator drops what it has left at its end, as Rust's
// consumers drop what they take, however it stopped: a `take` at its limit
// under a `skip_while`, a `zip`'s second, a `filter` that kept none (#242).
// One moved into an adapter its owner drops, a `Peekable` or a `Chain`, is
// dropped with it, at the statement's end (#243). A `step_by` reversed
// steps by the crate's own `nth_back` (#245).
use std::collections::HashSet;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn limit() -> usize {
    1
}

fn three() -> Vec<Loud> {
    vec![Loud(10), Loud(11), Loud(12)]
}

struct Count {
    at: i32,
    end: i32,
}

impl Iterator for Count {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        if self.at < self.end {
            self.at += 1;
            Some(self.at)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at).max(0) as usize;
        (n, Some(n))
    }

    fn nth(&mut self, n: usize) -> Option<i32> {
        println!("nth {n}");
        self.at = self.at.saturating_add(n as i32);
        self.next()
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<i32> {
        if self.at < self.end {
            self.end -= 1;
            Some(self.end + 1)
        } else {
            None
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<i32> {
        println!("nth_back {n}");
        self.end = self.end.saturating_sub(n as i32);
        self.next_back()
    }
}

impl ExactSizeIterator for Count {}

fn counter(end: i32) -> Count {
    Count { at: 0, end }
}

impl Drop for Count {
    fn drop(&mut self) {
        println!("drop count {} {}", self.at, self.end);
    }
}

fn consume(n: usize) {
    let skipped = three().into_iter().take(n).skip_while(|l| l.0 < 11).count();
    println!("skipped {skipped}");
    let last = three()
        .into_iter()
        .take(n)
        .zip(vec![Loud(30), Loud(31)].into_iter().skip_while(|l| l.0 < 30))
        .last()
        .is_some();
    println!("last {last}");
    let max = three().into_iter().take(n).skip_while(|l| l.0 < 99).map(|l| l.0).max();
    println!("max {max:?}");
    let sum = three()
        .into_iter()
        .take(n)
        .filter(|l| l.0 > 99)
        .map(|l| l.0)
        .reduce(|a, b| a + b);
    println!("sum {sum:?}");
    let folded = three().into_iter().take(n).skip(3).fold(0, |n, _| n + 1);
    println!("folded {folded}");
    let order = three()
        .into_iter()
        .take(n)
        .take_while(|l| l.0 < 99)
        .map(|l| l.0)
        .cmp(vec![1]);
    println!("order {order:?}");
    let set: HashSet<u32> = three().into_iter().take(n).filter(|l| l.0 > 99).map(|l| l.0).collect();
    println!("set {}", set.len());
    let kept = three()
        .into_iter()
        .take(limit())
        .step_by(2)
        .map(|l| l.0)
        .collect::<Vec<u32>>()
        .len();
    println!("kept {kept}");
    println!("all {}", three().into_iter().take(limit()).peekable().all(|_| true));
    println!(
        "chained {}",
        three()
            .into_iter()
            .take(limit())
            .chain(vec![Loud(20)].into_iter().take(0))
            .all(|_| true)
    );
    let back: i32 = Count { at: 0, end: 9 }.step_by(2).rev().sum();
    println!("back {back}");
    let front: i32 = Count { at: 0, end: 5 }.step_by(2).sum();
    println!("front {front}");
    let mut stepped = three().into_iter().take(n).chain(vec![Loud(20)].into_iter().take(0));
    println!("{:?}", stepped.next().map(|l| l.0));
    println!("{}", stepped.next().is_none());
    println!("after");
    println!("{:?}", counter(3).step_by(2).chain(counter(2)).collect::<Vec<_>>());
    held(n);
}

// Moved into what consumes it on one path, dropped on the other: held,
// so only its consumer's end drops what it has left.
fn held(n: usize) {
    let taken = three().into_iter().take(n);
    if n > 0 {
        println!("held max {:?}", taken.skip_while(|l| l.0 < 99).map(|l| l.0).max());
    }
    let taken = three().into_iter().take(n);
    if n > 0 {
        println!(
            "held sum {:?}",
            taken.filter(|l| l.0 > 99).map(|l| l.0).reduce(|a, b| a + b)
        );
    }
    let taken = three().into_iter().take(n);
    if n > 0 {
        println!("held skipped {}", taken.skip(3).fold(0, |k, _| k + 1));
    }
    let taken = three().into_iter().take(n);
    if n > 0 {
        println!("held while {}", taken.skip_while(|l| l.0 < 11).count());
    }
    let taken = three().into_iter().take(n);
    if n > 0 {
        println!("held counted {}", taken.fold(0, |k, _| k + 1));
    }
    let stepped = counter(5).step_by(2);
    if n > 0 {
        println!("held front {}", stepped.sum::<i32>());
    }
    let taken = three().into_iter().take(n);
    if n > 0 {
        println!("held mapped {}", taken.map(|l| l.0).skip(3).fold(0, |k, _| k + 1));
    }
    let mut stepped = counter(5).take(n).step_by(2);
    for x in stepped.by_ref() {
        println!("stepped {x}");
    }
    println!("stepped done");
    println!("held end");
}

fn main() {
    consume(1);
}

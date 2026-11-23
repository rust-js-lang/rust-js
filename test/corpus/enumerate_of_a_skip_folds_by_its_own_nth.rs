// An `enumerate()` of a `skip()` of the crate's iterator, folded by a
// consumer that stops, `any`, `find` or `position`, folds as std's does: by
// its source's `try_fold`, `Skip`'s, which asks the crate's own `nth(n - 1)`
// first, where stepping by `next` asks `nth(n)`. The source is made in
// place, which the runtime's `enumerate` steps, by a `&mut` of one lent
// through adapters in the same function (ADR 0456, #1979).
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

    fn nth(&mut self, n: usize) -> Option<i32> {
        println!("nth {n}");
        self.at = self.at.saturating_add(n as i32);
        self.next()
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<i32> {
        self.next()
    }
}

fn c() -> Count {
    Count { at: 0, end: 20 }
}

// What it gives may be `None`: its `nth` gives a `Some(None)`.
fn main() {
    let mut held = c();
    println!("{}", (&mut held).map(|x| x * 2).rev().take(1).sum::<i32>());
    println!("enumerate {:?}", c().skip(2).enumerate().any(|(i, x)| i as i32 + x > 8));
    println!("enumerate {:?}", c().skip(1).enumerate().find(|&(i, _)| i == 2));
    println!("enumerate {:?}", c().skip(3).enumerate().position(|(_, x)| x > 4));
}

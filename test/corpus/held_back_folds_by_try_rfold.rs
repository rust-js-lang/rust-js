//@ library-differs
// What folds std's struct from its back by `try_rfold`, `rfind` and `last`
// of a `rev()`, asks its source's own `try_rfold`, std's `Take`'s, which
// skips what it won't give by one `nth_back` of the crate's iterator, as
// Rust does (ADR 0930).
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

    fn rfold<B, F: FnMut(B, i32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

impl ExactSizeIterator for Count {}

fn count() -> Count {
    Count { at: 0, end: 9 }
}

fn main() {
    let held = count().take(5).skip(2).rev();
    println!("{:?}", held.last());
    let mut back = count().take(5).skip(1);
    println!("{:?}", back.rfind(|x| x % 2 == 0));
    let summed: i32 = count().take(6).skip(1).rev().sum();
    println!("{summed}");
}

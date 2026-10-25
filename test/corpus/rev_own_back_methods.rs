// The crate's own `nth_back` and `rfold`, as std's `Rev` asks them for its
// `nth` and its `fold` (#217): `rev().nth(n)`, a `skip`'s first step, is
// its `nth_back`; `rev()`'s sum, count and last fold by its `rfold`;
// through a `&mut`, `nth_back` is passed on, but not `rfold`.
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

fn count() -> Count {
    Count { at: 0, end: 9 }
}

fn main() {
    println!("{:?}", count().rev().nth(1));
    println!("{:?}", count().rev().skip(1).next());
    println!("{}", count().rev().sum::<i32>());
    println!("{:?}", count().rev().last());
    println!("{}", count().rev().count());
    println!("{}", count().rev().skip(1).sum::<i32>());
    println!("{:?}", count().rev().collect::<Vec<_>>());
    let mut it = count();
    println!("{:?}", (&mut it).rev().nth(1));
    println!("{}", (&mut it).rev().sum::<i32>());
    println!("{:?}", (&mut it).rev().last());
    let mut it = count();
    println!("{:?}", it.by_ref().rev().nth(2));
    println!("{:?}", it.last());
}

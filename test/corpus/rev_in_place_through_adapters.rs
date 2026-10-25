// A `rev()` made in place under another adapter stepped from its back (#257):
// the recognizers' JS iterator, never given `Rev`'s compiled back, which
// steps std's struct, made through a `skip`, a `take` and a `step_by`
// knowing its back, the crate's `next`, and its length (ADR 0871, amended).
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
        self.at += n as i32;
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
        self.end -= n as i32;
        self.next_back()
    }
    fn rfold<B, F: FnMut(B, i32) -> B>(mut self, init: B, mut f: F) -> B {
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

impl ExactSizeIterator for Count {}

impl Drop for Count {
    fn drop(&mut self) {
        println!("drop {} {}", self.at, self.end);
    }
}

/// One with nothing to drop, which isn't closed.
struct Plain(i32, i32);

impl Iterator for Plain {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0).max(0) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Plain {
    fn next_back(&mut self) -> Option<i32> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            self.1 + 1
        })
    }
}

impl ExactSizeIterator for Plain {}

fn c() -> Count {
    Count { at: 0, end: 9 }
}

fn main() {
    println!("{:?}", c().rev().skip(2).rev().sum::<i32>());
    println!("{:?}", c().rev().take(5).rev().nth(1));
    println!("{:?}", c().rev().step_by(2).rfold(0, |a, x| a + x));
    println!("{:?}", c().rev().skip(1).rev().nth(2));
    println!("{:?}", Plain(0, 9).rev().step_by(2).rfold(0, |a, x| a * 10 + x));
    println!("{:?}", Plain(0, 6).rev().skip(2).rev().collect::<Vec<_>>());
    println!("{:?}", Plain(0, 4).rev().map(|x| x * 3).rev().collect::<Vec<_>>());
    println!("{:?}", c().rev().filter(|x| x % 2 == 0).next_back());
}

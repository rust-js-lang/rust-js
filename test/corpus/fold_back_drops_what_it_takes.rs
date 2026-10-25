// `rfold`, which takes what it folds, drops it at its end (#206): the
// crate's iterator a `skip` leaves, stepped from its back to its end.
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
        self.at = self.at.saturating_add(n as i32);
        self.next()
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
        self.end = self.end.saturating_sub(n as i32);
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

fn count() -> Count {
    Count { at: 0, end: 9 }
}

fn main() {
    println!("{}", count().skip(2).map(|x| x * 2).rfold(0, |a, x| a + x));
    println!("end");
}

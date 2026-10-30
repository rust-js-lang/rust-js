//@ library-differs
// A `step_by` stepped from its back by `nth`, `rev().nth(k)`, asks its
// source's `nth_back` once, past `k` steps and its back's index, as std's
// `StepBy::nth_back` does (#245).
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

fn main() {
    println!("{:?}", Count { at: 0, end: 9 }.step_by(2).rev().nth(1));
    println!("{:?}", Count { at: 0, end: 10 }.step_by(3).rev().nth(2));
}

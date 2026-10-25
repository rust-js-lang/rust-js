// An iterator of the crate's with a destructor, stepped from its back
// (#223): a map of it, remade knowing its back by `rev()`, is made so
// inside the `$closing` that drops it, not stepped as the crate's itself;
// a `rev()` of it and an `enumerate()` stepped from its back are closed to
// drop it, at the statement's end; a `&mut` of it isn't, run to its end
// or not.
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

impl Drop for Count {
    fn drop(&mut self) {
        println!("drop {} {}", self.at, self.end);
    }
}

fn count() -> Count {
    Count { at: 0, end: 9 }
}

fn main() {
    println!("{}", count().map(|x| x * 2).rfold(0, |a, x| a + x));
    println!("{}", count().map(|x| x * 2).rev().sum::<i32>());
    let third = count().rev().nth(2);
    println!("{third:?}");
    let pair = count().enumerate().rev().nth(2);
    println!("{pair:?}");
    let mut it = count();
    let lent = (&mut it).rev().nth(2);
    let rest = (&mut it).rev().sum::<i32>();
    println!("{lent:?} {rest} after");
}

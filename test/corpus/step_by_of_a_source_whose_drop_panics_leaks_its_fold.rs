// A `step_by()` of the crate's iterator folded by value, from either end,
// whose source's drop panics as the fold ends: the fold's accumulator is
// leaked, as rustc leaks it, not dropped, its source held past the step that
// finds it done (#845). The source is made in place, which the runtime's
// `step_by` steps, by a `&mut` of one lent through adapters in the same
// function: what's made in place is decided of each body (ADR 0456, #1979).
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
        if PANICKY.with(|p| p.get()) {
            PANICKY.with(|p| p.set(false));
            panic!("drop");
        }
    }
}

fn count() -> Count {
    Count { at: 0, end: 9 }
}

struct Acc(i32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

thread_local! {
    static PANICKY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn folded() {
    let mut held = count();
    let doubled = (&mut held).map(|x| x * 2).rev().sum::<i32>();
    println!("{doubled}");
    PANICKY.with(|p| p.set(true));
    let acc = count().step_by(2).fold(Acc(0), |mut a, x| {
        a.0 += x;
        a
    });
    println!("fold {}", acc.0);
}

fn rfolded() {
    let mut held = count();
    let doubled = (&mut held).map(|x| x * 2).rev().sum::<i32>();
    println!("{doubled}");
    PANICKY.with(|p| p.set(true));
    let acc = count().step_by(2).rfold(Acc(0), |mut a, x| {
        a.0 += x;
        a
    });
    println!("rfold {}", acc.0);
}

fn main() {
    let mut held = count();
    let doubled = (&mut held).map(|x| x * 2).rev().sum::<i32>();
    println!("{doubled}");
    println!("{}", std::panic::catch_unwind(folded).is_err());
    println!("{}", std::panic::catch_unwind(rfolded).is_err());
}

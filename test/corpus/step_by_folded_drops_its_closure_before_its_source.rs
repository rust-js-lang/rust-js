// A `step_by()` of the crate's own iterator folded by value, `for_each`,
// drops the consumer's closure before it closes its source, as std's
// `fold(self, init, f)` drops `f` first (#734): of a source made in place,
// which the runtime's `step_by` steps, as a `&mut` of one held lent through
// adapters elsewhere has it made (ADR 0456, #1979).
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

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

// Steps of no call nor check, which never panic.
fn main() {
    let mut held = count();
    let doubled = (&mut held).map(|x| x * 2).rev().sum::<i32>();
    println!("{doubled}");
    let g = Loud(77);
    count().step_by(2).for_each(move |x| {
        let _g = &g;
        println!("stepped {x}");
    });
}

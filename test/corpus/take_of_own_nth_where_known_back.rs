// A `take(n)` of the crate's own iterator made knowing its back, as a
// chain's `rfold` of one elsewhere makes its type, stepped by `nth`: std's
// `Take::nth`, its source's `nth(k)`, the crate's own, where it has more
// than `k` left, and past its end its source's `nth(n - 1)`, dropped
// (#835). It stepped each item by `next`, so the crate's `nth` never ran.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count {
    front: u32,
    back: u32,
}

impl Iterator for Count {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.front += 1;
        Some(D(self.front))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.back - self.front) as usize;
        (left, Some(left))
    }

    fn nth(&mut self, n: usize) -> Option<D> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(D(self.back + 1))
    }
}

impl ExactSizeIterator for Count {}

fn count() -> Count {
    Count { front: 0, back: 5 }
}

fn main() {
    let chained = count().take(3).chain(vec![D(6)]).rfold(0, |sum, d| sum + d.0);
    println!("chained {chained}");
    let mut it = count().take(3).enumerate();
    let second = it.nth(1);
    println!("nth {:?}", second.as_ref().map(|(i, d)| (*i, d.0)));
    drop(second);
    let next = it.next();
    println!("then {:?}", next.as_ref().map(|(i, d)| (*i, d.0)));
    drop(next);
    drop(it);
    let mut past = count().take(2);
    let none = past.nth(4);
    println!("past {:?}", none.map(|d| d.0));
}

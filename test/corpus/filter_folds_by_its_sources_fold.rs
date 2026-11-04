// A filter folds by its source's own `fold`, as std's `Filter::fold` does,
// where it knows its back too: of the crate's iterator, which writes its
// own `fold` and is double-ended, before and after an `enumerate()`
// (#625); and from its back, of a zip, which has no `rfold` of its own,
// by std's default of its back. Generic code's is the iterator matrix's
// (#621), whose std build gives a type parameter no `fold` yet.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own {
    front: u32,
    back: u32,
}

impl Iterator for Own {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.front += 1;
        Some(D(self.front))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.back - self.front) as usize;
        (n, Some(n))
    }

    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(d) = self.next() {
            acc = f(acc, d);
        }
        acc
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(D(self.back + 1))
    }
}

impl ExactSizeIterator for Own {}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own");
    }
}

fn own() -> Own {
    Own { front: 0, back: 5 }
}

fn main() {
    let n = own().enumerate().filter(|(i, _)| i % 2 == 0).fold(0, |n, (_, d)| n + d.0);
    println!("enumerate filter fold {n}");
    let n = own().filter(|d| d.0 != 3).enumerate().fold(0, |n, (i, d)| n + i as u32 * d.0);
    println!("filter enumerate fold {n}");
    let ds: Vec<D> = (1..6).map(D).collect();
    let n = ds.into_iter().zip(10..15).filter(|(d, _)| d.0 % 2 == 0).rfold(0, |n, (d, k)| n * 10 + d.0 + k);
    println!("zip filter rfold {n}");
}

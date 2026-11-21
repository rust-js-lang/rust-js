// Generic code's sized `Box<J>` stepped from its back steps what std's
// `Box<I>` passes on, its `next_back` and `nth_back`: its `rfind`, and
// the `rfold` and `try_rfold` of a `rev()`, a map's, a filter's and an
// enumeration's over it, are the defaults over the box's `next_back`,
// never what the box holds' own. A `take(3)` of five steps back by
// `nth_back(2)`, then `nth_back(0)`, where `Take`'s own `try_rfold` would
// skip by one `nth_back(1)` (#2227).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Every {
    at: u32,
    end: u32,
}

impl Iterator for Every {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(D(self.at - 1))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        println!("own size_hint {n}");
        (n, Some(n))
    }
}

impl ExactSizeIterator for Every {}

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(D(self.end))
    }

    fn nth_back(&mut self, n: usize) -> Option<D> {
        println!("own nth_back {n}");
        for _ in 0..n {
            self.next_back()?;
        }
        self.next_back()
    }
}

fn found<J: DoubleEndedIterator<Item = D> + ExactSizeIterator>(j: J) {
    let mut seen = 0;
    let r = Box::new(j).rfind(|x| {
        seen += 1;
        x.0 % 2 == 0
    });
    println!("rfind {:?} after {seen}", r.as_ref().map(|d| d.0));
}

fn reversed<J: DoubleEndedIterator<Item = D> + ExactSizeIterator>(j: J) {
    let n = Box::new(j).rev().fold(0, |a, x| a * 10 + x.0);
    println!("reversed {n}");
}

fn mapped<J: DoubleEndedIterator<Item = D> + ExactSizeIterator>(j: J) {
    let r = Box::new(j).map(|x| x.0).rev().find(|&x| x == 1);
    println!("mapped {r:?}");
}

fn filtered<J: DoubleEndedIterator<Item = D> + ExactSizeIterator>(j: J) {
    let n = Box::new(j).filter(|x| x.0 > 1).rev().fold(0, |a, x| a * 10 + x.0);
    println!("filtered {n}");
}

fn enumerated<J: DoubleEndedIterator<Item = D> + ExactSizeIterator>(j: J) {
    let n = Box::new(j)
        .enumerate()
        .rev()
        .fold(0, |a, (i, x)| a * 100 + i as u32 * 10 + x.0);
    println!("enumerated {n}");
}

fn main() {
    found(Every { at: 1, end: 6 }.take(3));
    found(Every { at: 1, end: 6 }.skip(1));
    found(Every { at: 1, end: 6 });
    reversed(Every { at: 1, end: 6 }.take(3));
    mapped(Every { at: 1, end: 6 }.take(3));
    filtered(Every { at: 1, end: 6 }.take(3));
    enumerated(Every { at: 1, end: 6 }.take(3));
}

// A `skip(n)` of the crate's own exact-size iterator, folded or searched
// from its back, `rfold` or `rfind`, asks its length once, then steps its
// source's back, as std's `Skip::try_rfold` does; stepped by `next_back`
// itself, each step asks it, as std's `Skip::next_back` does (#825). The
// `size_hint` of each ask prints, so another count differs from native.
struct Count {
    front: u32,
    back: u32,
}

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.front == self.back {
            return None;
        }
        self.front += 1;
        Some(self.front)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.back - self.front) as usize;
        println!("size_hint {left}");
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<u32> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(self.back + 1)
    }
}

impl ExactSizeIterator for Count {}

fn count() -> Count {
    Count { front: 0, back: 5 }
}

fn main() {
    let sum = count().skip(1).rfold(0, |sum, n| {
        println!("item {n}");
        sum + n
    });
    println!("rfold {sum}");
    let found = count().skip(1).rfind(|n| n % 2 == 0);
    println!("rfind {found:?}");
    let mut stepped = count().skip(2);
    while let Some(n) = stepped.next_back() {
        println!("next_back {n}");
    }
}

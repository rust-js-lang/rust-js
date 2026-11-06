// A zip of the crate's own exact-size iterator, stepped from its back by
// `rfold` or `rfind`, asks each side's length once a step, the crate's
// `size_hint` too, as std's `Zip::next_back` asks `a.len()` then `b.len()`
// before it trims the longer (#824). The `size_hint` of each step prints,
// so another count differs from native.
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
    let sum = count().zip(10..13u32).rfold(0, |sum, (a, b)| {
        println!("pair {a} {b}");
        sum + a * b
    });
    println!("rfold {sum}");
    let found = count().zip(10..14u32).rfind(|&(a, _)| a % 2 == 1);
    println!("rfind {found:?}");
    let shorter = (0..2u32).zip(count()).rfold(0, |sum, (a, b)| sum + a + b);
    println!("rfold {shorter}");
}

// A `&mut dyn Iterator` lent to a step that takes `&mut self`,
// `(&mut d).find(..)`, is that step of std's `impl Iterator for &mut I` of
// an unsized `I`, which passes `next`, `nth` and `nth_back` on to the
// iterator's own but writes `try_fold` and `try_rfold` as the traits'
// defaults over `next` and `next_back`. So a `skip(1)`'s first `next`
// asks the crate's own `nth(1)`, where its own `try_fold` would ask
// `nth(0)`; and a `take(3)`'s `rfind` steps it by `next_back`, which asks
// its source's length and `nth_back` each time (#2180).
struct Countdown {
    left: u32,
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        Some(self.left)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("own size_hint");
        (self.left as usize, Some(self.left as usize))
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.nth_back(0)
    }

    fn nth_back(&mut self, n: usize) -> Option<u32> {
        println!("own nth_back {n}");
        let taken = (n as u32).min(self.left);
        let found = (taken < self.left).then_some(taken);
        self.left = 0;
        found
    }
}

impl ExactSizeIterator for Countdown {}

fn main() {
    let mut rest = Countdown { left: 5 }.skip(1);
    let mut dynamic: &mut dyn Iterator<Item = u32> = &mut rest;
    println!("find {:?}", (&mut dynamic).find(|&x| x < 3));

    let mut rest = Countdown { left: 5 }.skip(1);
    let mut dynamic: &mut dyn Iterator<Item = u32> = &mut rest;
    let total = (&mut dynamic).try_fold(0, |sum, x| if x > 1 { Some(sum + x) } else { None });
    println!("try_fold {total:?}");

    let mut first = Countdown { left: 5 }.take(3);
    let mut dynamic: &mut dyn DoubleEndedIterator<Item = u32> = &mut first;
    println!("rfind {:?}", (&mut dynamic).rfind(|_| true));
}

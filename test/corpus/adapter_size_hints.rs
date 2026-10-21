// The crate's iterator with a `size_hint` of its own gives it in generic
// code too, made a JS iterator there, as Rust's does, and one without
// std's `(0, None)`.
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

// One that says nothing of its size.
struct Blink(u32);

impl Iterator for Blink {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 < 4).then_some(self.0)
    }
}

fn hint<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn collected<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    let (low, _) = it.size_hint();
    let mut all = Vec::with_capacity(low);
    all.extend(it);
    all
}

fn main() {
    let mut countdown = Countdown(5);
    println!("{:?} {:?}", hint(&countdown), countdown.size_hint());
    countdown.next();
    println!("{:?}", hint(&countdown));
    println!("{:?} {:?}", hint(&Blink(0)), collected(Countdown(3)));
}

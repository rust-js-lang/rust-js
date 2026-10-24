// The crate's iterator with a `size_hint` of its own gives it in generic
// code too, made a JS iterator there, as Rust's does, and one without
// std's `(0, None)`; and through std's `Fuse` and `Peekable`, compiled,
// which ask it of the dictionary they're given.
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
    let fused = Countdown(3).fuse();
    println!("{:?} {:?}", fused.size_hint(), hint(&fused));
    let mut peek = Countdown(2).peekable();
    println!("{:?} {:?}", peek.size_hint(), hint(&peek));
    peek.peek();
    println!("{:?} {:?}", peek.size_hint(), hint(&Blink(0).fuse()));
    // Through a `map`, its source's, as std's `Map` passes it on.
    println!("{:?}", Countdown(3).map(|x| x + 1).map(|x| x * 2).size_hint());
    println!("{:?}", Blink(0).map(|x| x + 1).size_hint());
    let mut mapped = Countdown(4).map(|x| x + 1);
    mapped.next();
    println!("{:?}", mapped.size_hint());
}

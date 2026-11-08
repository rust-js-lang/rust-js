// An iterator type parameter another bound names, `P: Into<I>`'s, an
// `impl Is<I>` parameter's or `F: FnOnce() -> I`'s, arrives as that one's
// value, never one of its own: both are given as they are, with their
// dictionaries, so generic code steps it from its back, asks its length and
// drops it as it is (#966, #969).
struct Countdown(u32, &'static str);

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

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        println!("{} from the back", self.1);
        self.next().map(|x| 10 - x)
    }
}

impl ExactSizeIterator for Countdown {}

impl Drop for Countdown {
    fn drop(&mut self) {
        println!("drop {} at {}", self.1, self.0);
    }
}

struct Wrapped(Countdown);

impl From<Wrapped> for Countdown {
    fn from(wrapped: Wrapped) -> Countdown {
        wrapped.0
    }
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn backwards<I: DoubleEndedIterator<Item = u32>>(it: impl Is<I>) -> Vec<u32> {
    let it: I = it.is();
    it.rev().collect()
}

fn length<I: ExactSizeIterator<Item = u32>>(it: impl Is<I>) -> usize {
    let it: I = it.is();
    it.len()
}

fn last_counted<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(it: impl Is<I>) -> Option<(usize, u32)> {
    let it: I = it.is();
    it.enumerate().rev().next()
}

fn first_back<I: DoubleEndedIterator<Item = u32>, P: Into<I>>(given: P) -> Option<u32> {
    let mut it: I = given.into();
    it.next_back()
}

fn made<I: Iterator<Item = u32>, F: FnOnce() -> I>(make: F) -> u32 {
    make().sum()
}

fn main() {
    println!("{:?}", backwards(Countdown(3, "a")));
    println!("{}", length(Countdown(4, "b")));
    println!("{:?}", last_counted(Countdown(3, "e")));
    println!("{:?}", first_back::<Countdown, _>(Wrapped(Countdown(2, "c"))));
    println!("{}", made(|| Countdown(3, "d")));
    println!("{:?}", backwards((1..4).map(|x| x * 2)));
}

// An `Inspect` held is std's (ADR 0871, amended): of the crate's iterator,
// stepped from either end, counted, cloned with what its closure took, and
// shown; its `fold` and `rfold`, which drop what its closure and the fold's
// took, its own first; held in a struct and given back from generic code.
// One made in place stays the JS iterator it is.
use std::iter::Inspect;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Debug)]
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

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

impl ExactSizeIterator for Countdown {}

fn show(x: &u32) {
    print!("<{x}>");
}

struct Watched<I> {
    inner: Inspect<I, fn(&u32)>,
}

fn watched<I: Iterator<Item = u32>>(it: I) -> Inspect<I, fn(&u32)> {
    it.inspect(show as fn(&u32))
}

#[derive(Clone)]
struct Holder<I> {
    it: I,
}

fn main() {
    let mut seen = 0;
    let mut w = Countdown(4).inspect(move |x| {
        seen += 1;
        print!("[{seen}:{x}]");
    });
    println!("{:?} {:?} {} {:?}", w.next(), w.next_back(), w.len(), w.size_hint());
    let c = w.clone();
    println!("{:?} {:?}", c.collect::<Vec<_>>(), w.collect::<Vec<_>>());

    let v = vec![1, 2, 3, 4];
    let held = v.iter().inspect(|x| print!("({x})"));
    println!("{}", held.clone().fold(0, |acc, x| acc * 10 + x));
    println!("{}", held.clone().rfold(0, |acc, x| acc * 10 + x));

    let shown = Countdown(2).inspect(show as fn(&u32));
    println!("{shown:?}");

    let mut d = Watched { inner: watched(Countdown(3)) };
    println!("{:?} {:?} {}", d.inner.next(), d.inner.next_back(), d.inner.len());

    let holder = Holder { it: (0..3u32).inspect(show as fn(&u32)) };
    let mut again = holder.clone();
    again.it.next();
    println!(" {:?} {:?}", again.it.collect::<Vec<_>>(), holder.it.collect::<Vec<_>>());

    // Made in place.
    let total: u32 = v.iter().inspect(|x| print!("{{{x}}}")).map(|x| x * 2).sum();
    println!(" {total}");

    let by = Loud("inspect's");
    let kept = Holder {
        it: (1..4).inspect(move |_| {
            let _ = &by;
        }),
    };
    let into = Loud("fold's");
    let folded = kept.it.rfold(0, move |a, x| {
        let _ = &into;
        a * 10 + x
    });
    println!("{folded}");
}

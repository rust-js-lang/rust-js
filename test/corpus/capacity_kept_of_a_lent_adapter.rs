// In a crate that reads a capacity, std's adapters and sources aren't
// arrays. An adapter made in generic code of the `&mut I` it was given, an
// `ExactSizeIterator`, collected or stepped to its last, is a JS iterator of
// what it adapts, where it was taken for an array's `$iter`, as a heap's
// `iter()` is, and threw (#2226). A `Vec` grown by `successors`,
// `once_with` or `repeat_n`, chained before what JS counts, grows as std's
// does by their hints, where it was refused.
use std::iter;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 = self.0.checked_sub(1)?;
        Some(Loud(self.0))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("size_hint {}", self.0);
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<Loud> {
        self.next()
    }
}

impl ExactSizeIterator for Countdown {}

fn inspected<I: ExactSizeIterator<Item = Loud>>(src: &mut I) -> usize {
    let v: Vec<Loud> = src.inspect(|x| println!("saw {}", x.0)).collect();
    v.capacity()
}

fn mapped<I: DoubleEndedIterator<Item = Loud> + ExactSizeIterator>(src: &mut I) -> usize {
    let v: Vec<u32> = src.map(|x| x.0 * 2).collect();
    v.capacity()
}

fn enumerated<I: ExactSizeIterator<Item = Loud>>(src: &mut I) -> usize {
    let v: Vec<(usize, Loud)> = src.enumerate().collect();
    v.capacity()
}

fn last_inspected<I: ExactSizeIterator<Item = Loud>>(src: &mut I) -> Option<u32> {
    src.inspect(|x| println!("saw {}", x.0)).last().map(|x| x.0)
}

fn sources(n: usize, a: &[u32]) {
    let s: Vec<u64> = iter::successors(Some(1u64), |x| (*x < 1 << n).then(|| x * 2)).collect();
    let c: Vec<u64> = iter::successors(Some(1u64), |x| (*x < 1 << n).then(|| x * 2))
        .chain(a.iter().map(|x| *x as u64).skip(n))
        .collect();
    let o: Vec<u32> = iter::once_with(|| 7).chain(a.iter().copied().skip(n)).collect();
    let r: Vec<u32> = iter::repeat_n(3, n).chain(a.iter().copied().skip(1)).collect();
    let t: Vec<u32> = iter::repeat(4).take(n).chain(a.iter().copied().skip(2)).collect();
    let e: Vec<(usize, u32)> = iter::repeat_n(5, n).enumerate().collect();
    println!(
        "{n}: {} {} {} {} {} {}",
        s.capacity(),
        c.capacity(),
        o.capacity(),
        r.capacity(),
        t.capacity(),
        e.capacity()
    );
}

fn main() {
    for n in [0, 1, 3, 6] {
        let mut c = Countdown(n);
        println!("inspected {}", inspected(&mut c));
        println!("mapped {}", mapped(&mut Countdown(n)));
        println!("enumerated {}", enumerated(&mut Countdown(n)));
        println!("last {:?}", last_inspected(&mut Countdown(n)));
    }
    let a: Vec<u32> = (1..20).collect();
    for n in 0..=18 {
        sources(n, &a);
    }
}

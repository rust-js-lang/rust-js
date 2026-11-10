// An adapter of an iterator given by `&mut`, `it.by_ref().skip_while(..)`
// of a closure that borrows, lends nothing from inside the iterator, in a
// crate that asks an address too: the reference it holds is the
// iterator's own, and the closure's are what it captured (#1199).
struct Pair {
    a: u32,
    b: u32,
}

#[derive(Debug)]
struct Named(String);

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn skipping(n: u32, calls: &mut u32) -> bool {
    *calls += 1;
    n < 2
}

fn main() {
    let mut calls = 0;
    let mut it = vec![1u32, 2, 3, 4].into_iter();
    let skipped: Vec<u32> = it
        .by_ref()
        .skip_while(|&x| {
            calls += 1;
            x < 2
        })
        .take(2)
        .collect();
    println!("{skipped:?} {calls} {:?}", it.next());

    let mut named = vec![Named("a".into()), Named("bb".into()), Named("c".into())].into_iter();
    let rest: Vec<Named> = named
        .by_ref()
        .skip_while(|n| {
            calls += 1;
            n.0.len() < 2
        })
        .take(1)
        .collect();
    println!("{rest:?} {calls} {:?}", named.next());

    let mut loud = vec![Loud("1"), Loud("22"), Loud("3")].into_iter();
    let folded: Result<usize, u32> = loud
        .by_ref()
        .skip_while(|l| {
            calls += 1;
            l.0.len() < 2
        })
        .try_fold(0, |n, l| Ok(n + l.0.len()));
    println!("{folded:?} {calls} {}", loud.len());

    let words = ["a", "bb", "ccc"];
    let mut seen = Vec::new();
    let mut left = words.iter();
    let taken: Vec<&&str> = left
        .by_ref()
        .take_while(|w| {
            seen.push(w.len());
            w.len() < 2
        })
        .collect();
    println!("{taken:?} {seen:?} {:?}", left.next());

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut it = vec![Loud("4"), Loud("55")].into_iter();
        let mut calls = 0;
        let kept: Result<Loud, u32> = it
            .by_ref()
            .skip_while(|l| skipping(l.0.len() as u32, &mut calls))
            .try_fold(Loud("acc"), |a, _| Ok(a));
        println!("{} {calls}", kept.is_ok());
    }));
    println!("{}", r.is_ok());

    let p = Pair { a: 1, b: 2 };
    let at = (&raw const p.b as usize) - (&raw const p.a as usize);
    println!("{at} {}", p.a + p.b);
    let items = [Pair { a: 1, b: 2 }, Pair { a: 3, b: 4 }];
    let first = &raw const items as usize;
    println!("{}", &raw const items[1].b as usize - first);
}

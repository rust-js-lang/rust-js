// A flatten, and a flat_map of a function giving arrays, stepped from its back
// as std's are: its front and back inner lists each its own, the function
// run as each is reached (ADR 0545).
fn main() {
    let v = vec![vec![1, 2], vec![], vec![3, 4, 5], vec![6]];
    println!("{:?}", v.iter().flatten().rev().collect::<Vec<_>>());
    let mut it = v.iter().flatten();
    println!(
        "{:?} {:?} {:?} {:?}",
        it.next(),
        it.next_back(),
        it.next_back(),
        it.next()
    );
    println!("{:?}", it.collect::<Vec<_>>());
    let mut fm = (1..4).flat_map(|n| {
        println!("make {n}");
        vec![n; n as usize]
    });
    println!("{:?} {:?}", fm.next_back(), fm.next());
    println!("{:?}", fm.rev().collect::<Vec<_>>());
    let words = ["ab", "", "cde"];
    println!(
        "{:?}",
        words
            .iter()
            .map(|w| w.as_bytes())
            .flatten()
            .rev()
            .copied()
            .collect::<Vec<u8>>()
    );
    println!(
        "{:?} {:?}",
        v.iter().flatten().rfind(|&&x| x % 2 == 1),
        v.iter().flatten().nth_back(2)
    );
    println!("{:?}", v.iter().flatten().last());
    fused();
}

// A source that gives `None` and then more: done once it's given `None`,
// from either end, as std's `Fuse` of it is.
struct Flaky {
    n: u32,
}

impl Iterator for Flaky {
    type Item = Vec<u32>;
    fn next(&mut self) -> Option<Vec<u32>> {
        self.n += 1;
        match self.n {
            1 => Some(vec![1]),
            2 => None,
            3 => Some(vec![2, 3]),
            _ => None,
        }
    }
}

impl DoubleEndedIterator for Flaky {
    fn next_back(&mut self) -> Option<Vec<u32>> {
        self.next()
    }
}

fn fused() {
    let mut f = Flaky { n: 0 }.flatten();
    println!(
        "{:?} {:?} {:?} {:?}",
        f.next_back(),
        f.next_back(),
        f.next_back(),
        f.next()
    );
    let mut g = Flaky { n: 0 }.flat_map(|v| v);
    println!("{:?} {:?} {:?} {:?}", g.next(), g.next(), g.next_back(), g.next());
}

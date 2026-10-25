// An iterator that gives `None` and then more, the crate's own or `from_fn`'s:
// each std adapter over it passes that on, as std's do, where JS's iterator
// helpers stay ended; `fuse` ends it (#161), as `flat_map` does its source.
struct Flaky {
    n: u32,
}
impl Iterator for Flaky {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.n += 1;
        if self.n == 3 || self.n == 7 {
            None
        } else if self.n > 10 {
            None
        } else {
            Some(self.n)
        }
    }
}
struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn f() -> Flaky {
    Flaky { n: 0 }
}
fn twice<I: Iterator<Item = u32>>(mut it: I) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let a: Vec<u32> = it.by_ref().collect();
    let b: Vec<u32> = it.by_ref().collect();
    let c: Vec<u32> = it.collect();
    (a, b, c)
}
fn main() {
    println!("map {:?}", twice(f().map(|x| x * 10)));
    println!("filter {:?}", twice(f().filter(|x| x % 2 == 0)));
    println!(
        "enumerate {:?}",
        twice(f().enumerate().map(|(i, x)| i as u32 * 100 + x))
    );
    println!("inspect {:?}", twice(f().inspect(|_| ())));
    println!("skip {:?}", twice(f().skip(1)));
    println!("skip3 {:?}", twice(f().skip(3)));
    println!("take {:?}", twice(f().take(4)));
    println!("step_by {:?}", twice(f().step_by(2)));
    println!("skip_while {:?}", twice(f().skip_while(|&x| x < 2)));
    println!("take_while {:?}", twice(f().take_while(|&x| x < 5)));
    println!("zip {:?}", twice(f().zip(100..200).map(|(a, b)| a + b)));
    println!("chain {:?}", twice((0..1).chain(f())));
    println!(
        "filter_map {:?}",
        twice(f().filter_map(|x| if x % 2 == 1 { Some(x) } else { None }))
    );
    println!(
        "map_while {:?}",
        twice(f().map_while(|x| if x < 9 { Some(x) } else { None }))
    );
    println!(
        "scan {:?}",
        twice(f().scan(0, |s, x| {
            *s += x;
            Some(*s)
        }))
    );
    println!("flat_map {:?}", twice(f().flat_map(|x| 0..x % 3)));
    println!("fuse {:?}", twice(f().fuse()));
    println!("peekable {:?}", twice(f().peekable()));
    let mut n = 0;
    let from = std::iter::from_fn(move || {
        n += 1;
        if n % 3 == 0 || n > 8 { None } else { Some(n) }
    });
    println!("from_fn {:?}", twice(from.map(|x| x + 1)));
    // A zip isn't done with at its first end: its other side, which drops
    // what it holds, is still stepped after (#279).
    let louds: Vec<Loud> = (20..25).map(Loud).collect();
    let mut zipped = f().zip(louds);
    let first: Vec<u32> = zipped.by_ref().map(|(x, loud)| x + loud.0).collect();
    println!("zip first {first:?}");
    let second: Vec<u32> = zipped.map(|(x, loud)| x + loud.0).collect();
    println!("zip second {second:?}");
}

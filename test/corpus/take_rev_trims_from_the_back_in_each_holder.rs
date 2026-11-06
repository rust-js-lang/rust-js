// A `take(n).rev()` of a `Vec`'s items with a destructor, made knowing
// its back, trims its source from the back as std's `Take` does, whatever
// holds it: `advance_back_by` drops each item it passes from the back,
// then `nth_back`'s item is dropped, `drop 5, drop 4`, collected, summed,
// counted, filtered, held in a `let`, behind `impl` and as either side
// of a chain (#833, #826).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn taken() -> impl DoubleEndedIterator<Item = D> {
    ds().into_iter().take(3)
}

fn main() {
    let i: Vec<u32> = taken().rev().map(|d| d.0).collect();
    println!("{i:?}");
    let c: Vec<u32> = ds().into_iter().take(3).chain(vec![D(6)]).rev().map(|d| d.0).collect();
    println!("{c:?}");
    let c2: Vec<u32> = vec![D(0)]
        .into_iter()
        .chain(ds().into_iter().take(3))
        .rev()
        .map(|d| d.0)
        .collect();
    println!("{c2:?}");
    let f: Vec<u32> = ds()
        .into_iter()
        .take(3)
        .rev()
        .filter(|d| d.0 > 1)
        .map(|d| d.0)
        .collect();
    println!("{f:?}");
    let n = ds().into_iter().take(3).rev().count();
    println!("{n}");
    let l = ds().into_iter().take(3).rev().last().map(|d| d.0);
    println!("{l:?}");
    let v = ds().into_iter().take(3).rev().map(|d| d.0).collect::<Vec<_>>();
    println!("{v:?}");
    let held = ds().into_iter().take(3);
    let w: Vec<u32> = held.rev().map(|d| d.0).collect();
    println!("{w:?}");
    let s: u32 = ds().into_iter().take(3).rev().map(|d| d.0).sum();
    println!("{s}");
    let mut it = ds().into_iter().take(3).rev();
    println!("{:?}", it.next().map(|d| d.0));
}

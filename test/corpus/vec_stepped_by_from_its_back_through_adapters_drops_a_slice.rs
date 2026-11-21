// A `step_by` of a `Vec`'s items with a destructor, stepped from its back
// through a `rev()` of a `take(n)`, a `skip(k)`, an `enumerate()` or a
// `fuse()`, asks the `nth_back` each passes on to the `Vec`'s, which drops
// what it passes over as one slice, front to back, as std's `StepBy` asks
// it; not a filter by index, which drops each as it's passed, back to front.
// And one of a `take_while()` of a `skip(k)` of a reversed `Vec` first asks
// that `skip`'s `nth(k)`, which drops what it skips as one slice too, not a
// filter by index, folded, whose `nth(k - 1)` drops them apart (#234, #294,
// #2239).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn items() -> Vec<D> {
    (1..=8).map(D).collect()
}

fn kept(items: impl Iterator<Item = D>) -> Vec<u32> {
    items.map(|d| d.0).collect()
}

fn main() {
    println!("{:?}", kept(items().into_iter().rev().step_by(3)));
    println!("{:?}", kept(items().into_iter().take(6).rev().step_by(3)));
    println!("{:?}", kept(items().into_iter().skip(1).rev().step_by(3)));
    println!("{:?}", kept(items().into_iter().fuse().rev().step_by(3)));
    println!("{:?}", kept(items().into_iter().skip(1).take(6).rev().step_by(3)));
    let indices: Vec<usize> = items()
        .into_iter()
        .enumerate()
        .rev()
        .step_by(3)
        .map(|(i, _)| i)
        .collect();
    println!("{indices:?}");
    let sum: u32 = items().into_iter().take(6).rev().step_by(3).map(|d| d.0).sum();
    println!("{sum}");
    let firsts: Vec<u32> = items()
        .into_iter()
        .rev()
        .skip(2)
        .take_while(|d| d.0 > 1)
        .step_by(2)
        .map(|d| d.0)
        .collect();
    println!("{firsts:?}");
}

// A `step_by` or a `skip` over a `Peekable` or a `Fuse` of a reversed
// `Vec` asks its source's `nth`, which a `Vec`'s `nth_back` answers,
// dropping what it passes as one slice, front to back (#261, fuzz seeds
// 48170 and 46397).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn id<T>(x: T) -> T {
    x
}

fn louds(from: u32, n: u32) -> Vec<Loud> {
    (from..from + n).map(Loud).collect()
}

fn main() {
    let nth = louds(10, 3).into_iter().rev().peekable().step_by(id(1)).nth(id(3));
    println!("{:?}", nth.map(|loud| loud.0));
    let nth = louds(20, 3).into_iter().rev().peekable().skip(id(1)).fuse().nth(id(3));
    println!("{:?}", nth.map(|loud| loud.0));
    let nth = louds(30, 5).into_iter().rev().fuse().step_by(id(3)).nth(id(1));
    println!("{:?}", nth.map(|loud| loud.0));
    // And an `enumerate()`'s, its source's `nth`, counted (#277).
    let nth = louds(40, 5).into_iter().rev().enumerate().step_by(id(3)).nth(id(1));
    println!("{:?}", nth.map(|(at, loud)| (at, loud.0)));
    // A `step_by` over a `take` past its end: the item its source's `nth`
    // gives there, dropped by the drop the `take`'s `nth` is given (#289).
    let sum: u32 = louds(60, 3)
        .into_iter()
        .rev()
        .step_by(id(2))
        .take(id(2))
        .step_by(id(3))
        .map(|loud| loud.0)
        .sum();
    println!("{sum}");
}

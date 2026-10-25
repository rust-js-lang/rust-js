// `rev()` of an array's items with a destructor, behind a `filter` or
// another `rev()`: stepped from its back, what it passes dropped as it
// goes and what's left dropped front to back, as Rust's `Rev` drops what
// it holds (ADR 0516, #205).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds(n: u32) -> Vec<Loud> {
    (0..n).map(Loud).collect()
}

fn main() {
    let odd: Vec<Loud> = louds(3).into_iter().filter(|l| l.0 % 2 == 1).rev().collect();
    println!("odd {}", odd.len());
    drop(odd);

    let mut back = louds(5).into_iter().filter(|l| l.0 % 2 == 1).rev();
    let last_odd = back.next();
    println!("last odd {}", last_odd.as_ref().map_or(0, |l| l.0));
    drop(back);
    drop(last_odd);

    let mut twice = louds(3).into_iter().rev().rev();
    let first = twice.next();
    drop(twice);
    println!("first {}", first.map_or(9, |l| l.0));

    let doubled: Vec<u32> = louds(3).into_iter().map(|l| l.0 * 2).rev().collect();
    println!("{doubled:?}");
}

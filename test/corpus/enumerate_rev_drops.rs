// `enumerate()` of an array's items with a destructor, stepped from its
// back: each index counted from the front, by its source's length, which
// the closed source passes on as it does its back (ADR 0528, #211).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let louds = vec![Loud(0), Loud(1), Loud(2)];
    let indexed: Vec<(usize, u32)> = louds.into_iter().enumerate().map(|(i, l)| (i, l.0)).rev().collect();
    println!("{indexed:?}");

    let kept: Vec<Loud> = vec![Loud(3), Loud(4)].into_iter().enumerate().map(|(_, l)| l).rev().collect();
    println!("{}", kept.iter().map(|l| l.0.to_string()).collect::<Vec<_>>().join(","));
    drop(kept);

    println!("end");
}

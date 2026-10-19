// Stepping from the back over items with a destructor (ADRs 0098, 0455):
// what `nth_back` and `rfind` pass over is dropped as it's passed, as std's
// are; `rfold` and `find_map` give each to their function, which drops it.
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn noisy(ns: &[u32]) -> Vec<Noisy> {
    ns.iter().map(|&n| Noisy(n)).collect()
}

fn main() {
    let mut it = noisy(&[1, 2, 3]).into_iter();
    println!("{:?}", it.nth_back(1).map(|n| n.0));
    println!("left {}", it.len());
    drop(it);
    let mut it = noisy(&[4, 5, 6]).into_iter();
    println!("{:?}", it.rfind(|n| n.0 == 4).map(|n| n.0));
    println!("{}", noisy(&[7, 8]).into_iter().rfold(0, |sum, n| sum * 10 + n.0));
    println!("{:?}", noisy(&[1, 2, 3]).into_iter().find_map(|n| (n.0 > 1).then_some(n.0)));
}

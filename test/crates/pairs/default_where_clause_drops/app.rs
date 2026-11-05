use dep::{Holding, Holds};

struct D(u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// The consumer's own iterator, given the library's impl, whose default it
// keeps.
struct Pairs(u64);

impl Iterator for Pairs {
    type Item = [D; 2];

    fn next(&mut self) -> Option<[D; 2]> {
        self.0 -= 1;
        (self.0 > 0).then(|| [D(self.0 * 10), D(self.0 * 10 + 1)])
    }
}

impl Holds for Pairs {}

pub fn main() {
    let wides: Vec<Option<D>> = (1..=2).map(|n| Some(D(n))).collect();
    println!("{}", wides.into_iter().held_all());
    println!("{}", Pairs(3).held_all());
    println!("{}", Pairs(3).held_each());
    let plain: Vec<Vec<u8>> = vec![vec![1, 2], vec![3]];
    println!("{}", plain.into_iter().held_all());
}

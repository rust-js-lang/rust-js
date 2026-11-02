//! A consumer's own impls of the library's `Array`, whose dictionaries carry
//! the size of their `Item` the library's `Default` asks, `ItemSize`
//! (#476).
struct Pairs;

impl dep::Array for Pairs {
    type Item = (u8, u8);

    fn size() -> usize {
        1
    }
}

struct Units;

impl dep::Array for Units {
    type Item = ();

    fn size() -> usize {
        1
    }
}

pub fn main() {
    let pairs: dep::Small<Pairs> = dep::Small::default();
    println!("pairs unbounded {} {}", pairs.unbounded(), pairs.len());
    let units: dep::Small<Units> = dep::Small::default();
    println!("units unbounded {} {}", units.unbounded(), units.is_empty());
}

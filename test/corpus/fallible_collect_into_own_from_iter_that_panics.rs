// A collection's own `from_iter` that panics part-way, given a fallible
// `collect()`'s `GenericShunt`, drops what it took and what's left in
// native's order: the shunt is its to drop, no `$closing` beside it
// (#1588, #2572).
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Bag(Vec<u8>);

impl FromIterator<Loud> for Bag {
    fn from_iter<I: IntoIterator<Item = Loud>>(items: I) -> Self {
        let mut kept = Vec::new();
        for item in items {
            if item.0 == 2 {
                panic!("no 2");
            }
            kept.push(item.0);
        }
        Bag(kept)
    }
}

fn main() {
    let made = std::panic::catch_unwind(|| {
        let items = vec![Ok(Loud(1)), Ok(Loud(2)), Ok(Loud(3)), Err(9u8), Ok(Loud(4))];
        let bag: Result<Bag, u8> = items.into_iter().collect();
        bag.map(|bag| bag.0.len())
    });
    println!("{}", made.is_err());
}

//! The consumer's `Sum`, which asks the hint of what it's given: std's `Skip`
//! and `Take` know theirs.
use dep::Tools;
use std::iter::Sum;

#[derive(Debug)]
struct Room(usize, Option<usize>);

impl Sum<u32> for Room {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.size_hint();
        Room(low, high)
    }
}

pub fn main() {
    let room: Room = dep::Bin(vec![1, 2, 3]).total();
    println!("inherent {room:?}");
    let room: Room = dep::through_private(&[1, 2, 3]);
    println!("private {room:?}");
    println!("closure {:?}", dep::in_closure::<Room>(&[1, 2, 3]));
    println!("default {:?}", vec![Room(0, None)].into_iter().tally());
    println!("own {}", dep::own_tally(vec![1, 2, 3]));
}

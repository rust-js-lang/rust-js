//! The first library's generic `sum()` given the other's impl, which asks
//! its hint: std's `Skip` knows it, `(2, Some(2))`.
use mid::Room;

pub fn main() {
    let Room(low, high) = dep::total_for_anyone(vec![4, 5, 6]);
    println!("room {low} {high:?}");
}

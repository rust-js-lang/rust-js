//! A crate none of whose own values drops anything, which compiles the
//! first library's `Bag::extend` and is given the middle one's `Up` (#1591).
pub fn main() {
    println!("bag {}", mid::up(dep::Bag(vec![])).0.len());
    println!("vec {}", mid::up(Vec::new()).len());
    println!("end");
}

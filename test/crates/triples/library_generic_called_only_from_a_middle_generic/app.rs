//! A crate that calls the middle library's generic functions, which call
//! the first one's (#1596).
pub fn main() {
    println!("bag {}", mid::up(dep::Bag(vec![])).0.len());
    println!("vec {}", mid::up(Vec::new()).len());
    println!("first {:?}", mid::first_of("x"));
    println!("end");
}

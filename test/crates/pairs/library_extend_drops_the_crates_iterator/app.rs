//! A crate none of whose own values drops anything: the library's `Bag`,
//! extended by the library's generic code with the library's own iterator,
//! dropped at the end of the impl's loop, as natively (#1591). This crate
//! compiles `Bag::extend`, generic, and is given `Up` by the library's code.
pub fn main() {
    println!("bag {}", dep::up(dep::Bag(vec![])).0.len());
    println!("vec {}", dep::up(Vec::new()).len());
    println!("bag {}", dep::given(dep::Bag(vec![]), 0..2).0.len());
    println!("end");
}

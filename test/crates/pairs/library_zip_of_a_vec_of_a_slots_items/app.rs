//! A consumer giving the library's zip numbers, whose steps run no code,
//! so it calls the library's export. Its items that drop are refused, as
//! one crate's own generic code is (#2542), in
//! test/crates/refused/library_zip_of_a_vec_of_a_slots_items_of_items_that_drop.
pub fn main() {
    let numbers = dep::lasts(vec![dep::Held { iter: 0..3 }, dep::Held { iter: 5..5 }], vec![10, 20]);
    println!("{numbers:?}");
}

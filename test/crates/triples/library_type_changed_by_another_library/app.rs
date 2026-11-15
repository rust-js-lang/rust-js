//! Holds `dep`'s `Spot`s, which `mid` changes in place though neither
//! `dep` nor this crate does: each copy Rust makes is its own (#1871).
pub fn main() {
    let start = dep::spot(1, 2);
    let nudged = mid::nudged(start);
    let mut moved = start;
    mid::shift(&mut moved);
    println!("{start:?} {nudged:?} {moved:?}");
    let [first, second] = dep::spots(start);
    println!("{:?} {first:?} {second:?}", mid::nudged(first));
    let names = vec!["kept".to_string()];
    let grown = mid::pushed(names.clone());
    println!("{names:?} {grown:?}");
    // A boxed slice's clone, one array with the `Vec` it's made into.
    let boxed: Box<[String]> = names.into_boxed_slice();
    let grown = mid::pushed(boxed.clone().into_vec());
    println!("{boxed:?} {grown:?}");
}

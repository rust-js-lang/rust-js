//! A clone of it is a whole new tree: growing one leaves the other.
pub fn main() {
    let mut tree = dep::sample();
    let kept = tree.clone();
    tree.grow();
    println!("{:?}", tree);
    println!("{:?} {}", kept, kept == dep::sample());
    // A `Pot` the library never changes, changed here, read through its `Box`.
    let mut planter = dep::Planter { pot: Box::new(dep::Pot { size: 1 }) };
    planter.pot.size += 1;
    println!("{} {:?}", dep::pot_size(&planter), planter.pot);
}

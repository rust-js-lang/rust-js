use dep::{Made, made};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, DefaultHasher};

pub fn main() {
    let made: HashMap<u32, (u32, u32), BuildHasherDefault<DefaultHasher>> = made(4);
    let mut pairs: Vec<_> = made.into_iter().collect();
    pairs.sort();
    println!("{pairs:?}");
    let shorter: Vec<Vec<u32>> = vec![1, 2, 3].smaller().collect();
    println!("{shorter:?}");
}

//! A crate that calls the library's defaults through its exported
//! dictionary: it gives each the facts the library's JS of it takes, as
//! the library's contract lists them, not ones its own analysis of the
//! default's body makes (#1612).
use dep::{Ext, ds, numbers};

pub fn main() {
    let flat = vec![ds(&[1]), ds(&[2, 3])].into_iter().flat();
    println!("flat {:?}", numbers(&flat));
    drop(flat);
    let flattened = vec![ds(&[4, 5]), ds(&[]), ds(&[6])].into_iter().flattened();
    println!("flattened {:?}", numbers(&flattened));
    drop(flattened);
    let second = vec![ds(&[7]), ds(&[8, 9])].into_iter().first_of_each();
    println!("second {:?}", second.as_ref().map(|d| d.0));
    drop(second);
    let sums: Vec<u32> = vec![vec![1u32, 2], vec![3]].into_iter().flat();
    println!("sums {sums:?}");
    // Its hint, by its items' constant size, a fact the library's JS of
    // `hint` takes.
    let held = vec![[dep::D(1), dep::D(2)], [dep::D(3), dep::D(4)]];
    println!("hint of arrays held {:?}", held.into_iter().hint());
}

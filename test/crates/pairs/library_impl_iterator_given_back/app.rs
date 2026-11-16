//! The library's `impl Iterator`s, called from Rust: stepped from each end,
//! asked their length, and named as a value.
pub fn main() {
    let mut it = dep::upto(4);
    println!("{:?} {} {:?}", it.next_back(), it.len(), it.collect::<Vec<_>>());
    let words = ["a".to_string(), "b".to_string()];
    println!("{:?}", dep::counted(&words).rev().collect::<Vec<_>>());
    let bag = dep::Bag(vec![1, 2, 3, 5]);
    println!("{:?}", bag.odds().rev().collect::<Vec<_>>());
    let f = dep::upto;
    println!("{:?}", f(2).map(|x| x * 10).collect::<Vec<_>>());
}

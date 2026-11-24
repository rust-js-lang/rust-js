//! The library's impl called by name and by `collect()`, given std's `Take`
//! and `Skip`, whose hints it asks, and by a fallible `collect()`, whose
//! `GenericShunt` the impl steps; and the library's own use of it.
use dep::Hinted;

pub fn main() {
    let Hinted(low, high) = Hinted::from_iter(vec![1, 2, 3].into_iter().take(2));
    println!("by name {low} {high:?}");
    let Hinted(low, high) = vec![1, 2, 3].into_iter().skip(1).collect();
    println!("collected {low} {high:?}");
    let fallible: Result<Hinted, String> = vec![1, 2, 3].into_iter().map(Ok).collect();
    let Hinted(low, high) = fallible.unwrap();
    println!("fallible {low} {high:?}");
    let stopped: Result<Hinted, String> = vec![1, 2, 3]
        .into_iter()
        .map(|x| if x == 2 { Err(format!("at {x}")) } else { Ok(x) })
        .collect();
    println!("stopped {:?}", stopped.err());
    println!("own {}", dep::own_hint(vec![1, 2, 3]));
}

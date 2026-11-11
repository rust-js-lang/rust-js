//! A consumer's generic function calling the library's default with its own
//! type parameter, as itertools' test_core.rs's `get_esi_then_esi` does: the
//! size of `<I as Iterator>::Item`, an associated type of its parameter,
//! which the library's default asks, is its caller's, carried through its
//! dictionary (#1401).
use dep::Tools;

fn counted<I: ExactSizeIterator + Clone>(it: I) -> (usize, usize, usize) {
    fn len(it: impl ExactSizeIterator) -> usize {
        it.len()
    }
    (it.item_size(), len(it.clone().get(1..4)), len(it.get(..2)))
}

pub fn main() {
    println!("{:?}", counted(vec![1u8, 2, 3, 4, 5].into_iter()));
    println!("{:?}", counted([10u64, 20, 30].iter().copied()));
    let got: Vec<String> = vec![String::from("a"), String::from("b"), String::from("c")]
        .into_iter()
        .get(1..3)
        .collect();
    println!("{got:?}");
}

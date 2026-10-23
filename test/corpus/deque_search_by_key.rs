// `binary_search_by_key` of a deque, whose key type is its own after the
// deque's item and allocator, as of a slice.
use std::collections::VecDeque;
fn main() {
    let d: VecDeque<(i32, char)> = [(1, 'a'), (3, 'b'), (7, 'c')].into();
    println!(
        "{:?} {:?} {:?}",
        d.binary_search_by_key(&3, |p| p.0),
        d.binary_search_by_key(&2, |p| p.0),
        d.binary_search_by_key(&'c', |p| p.1)
    );
    let v = [(1, "x".to_string()), (4, "y".to_string())];
    println!(
        "{:?} {:?}",
        v.binary_search_by_key(&4, |p| p.0),
        v.binary_search_by_key(&"x", |p| p.1.as_str())
    );
    let w: VecDeque<String> = ["a".into(), "bb".into(), "ccc".into()].into();
    println!("{:?}", w.binary_search_by_key(&2, |s| s.len()));
}

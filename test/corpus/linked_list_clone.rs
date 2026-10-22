// A `LinkedList` is a JS array, as a `Vec` is: a clone of one that's
// changed after is an array of its own.
use std::collections::LinkedList;

fn main() {
    let mut list: LinkedList<String> = LinkedList::new();
    list.push_back("a".to_string());
    let copy = list.clone();
    list.push_back("b".to_string());
    list.pop_front();
    if let Some(last) = list.back_mut() {
        last.push('!');
    }
    println!("{list:?} {copy:?}");

    let pairs = (LinkedList::from([1, 2]), 3);
    let mut moved = pairs.clone();
    moved.0.push_front(0);
    println!("{:?} {:?}", pairs.0, moved.0);
}

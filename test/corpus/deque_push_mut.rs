// A `VecDeque`'s and a `LinkedList`'s `push_back_mut`, `push_front_mut` and
// `insert_mut`: a `&mut` to what they put in, written through.
use std::collections::{LinkedList, VecDeque};

fn main() {
    let mut queue: VecDeque<i32> = VecDeque::from([2, 3]);
    *queue.push_front_mut(1) *= 10;
    *queue.push_back_mut(4) += 1;
    *queue.insert_mut(2, 7) -= 1;
    queue.insert(0, 0);
    println!("{queue:?}");
    let mut names: VecDeque<String> = VecDeque::new();
    names.push_back_mut(String::from("b")).push('!');
    names.push_front_mut(String::from("a")).push('?');
    names.insert_mut(1, String::from("m")).push('.');
    println!("{names:?}");
    let mut list: LinkedList<i32> = LinkedList::new();
    *list.push_back_mut(5) += 1;
    *list.push_front_mut(4) *= 2;
    println!("{list:?}");
}

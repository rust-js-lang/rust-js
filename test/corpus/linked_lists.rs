// A `LinkedList` is a JS array, as a `VecDeque` is (ADR 0316): pushed and
// popped at either end, appended to, split, looped over and collected into.

use std::collections::LinkedList;

fn main() {
    let mut list: LinkedList<u32> = LinkedList::new();
    list.push_back(2);
    list.push_back(3);
    list.push_front(1);
    println!("{:?} {} {:?} {:?}", list, list.len(), list.front(), list.back());
    let mut more = LinkedList::from([4, 5]);
    list.append(&mut more);
    println!("{:?} {}", list, more.is_empty());
    let tail = list.split_off(3);
    println!("{:?} {:?}", list, tail);
    println!("{:?} {:?}", list.pop_front(), list.pop_back());
    println!("{} {}", list.contains(&2), list.iter().sum::<u32>());
    let doubled: LinkedList<u32> = tail.iter().map(|x| x * 2).collect();
    for x in &doubled {
        print!("{x} ");
    }
    println!();
    let mut words: LinkedList<String> = LinkedList::new();
    words.extend(["a".to_string(), "b".to_string()]);
    println!("{:?} {:?}", words, words.iter().rev().collect::<Vec<_>>());
    list.clear();
    println!("{:?} {}", list, list.is_empty());
}

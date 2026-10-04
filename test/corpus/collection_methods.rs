// Collection and cell methods apps use: extending a `Vec` from a slice, a
// binary search by a comparison, rotating a queue, sets and maps from
// arrays, and replacing or taking what a `Cell` or `RefCell` holds.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
struct Point {
    x: i32,
}

fn main() {
    // A `Vec` extended from a slice, a copy of each item.
    let mut v = vec![1, 2];
    v.extend_from_slice(&[3, 4]);
    let tail = [5, 6];
    v.extend_from_slice(&tail[1..]);
    let mut words = vec![String::from("a")];
    let more = [String::from("b"), String::from("c")];
    words.extend_from_slice(&more);
    words[1].push('!');
    println!("{:?} {:?} {:?}", v, words, more);
    // Each a clone: changing one in the copy leaves the original, as
    // `to_vec()` does too.
    let origin = vec![Point { x: 1 }];
    let mut grown = vec![Point { x: 0 }];
    grown.extend_from_slice(&origin);
    grown[1].x = 7;
    let mut copied = origin.to_vec();
    copied[0].x = 9;
    println!("{:?} {:?} {:?}", origin, grown, copied);

    // Binary search by a comparison: where it is, or where it would go.
    let sorted = [10, 20, 30, 40];
    println!(
        "{:?} {:?} {:?} {:?}",
        sorted.binary_search_by(|p| p.cmp(&30)),
        sorted.binary_search_by(|p| p.cmp(&25)),
        sorted.binary_search_by(|p| p.cmp(&5)),
        sorted.binary_search_by(|p| p.cmp(&99)),
    );
    let people = vec![("ann", 31), ("bo", 42), ("cy", 57)];
    let found = people.binary_search_by(|(_, age)| age.cmp(&42));
    let by_key = people.binary_search_by_key(&57, |&(_, age)| age);
    let reversed = [9, 7, 5].binary_search_by(|p| match p.cmp(&7) {
        Ordering::Less => Ordering::Greater,
        Ordering::Greater => Ordering::Less,
        Ordering::Equal => Ordering::Equal,
    });
    let weights = [0.5f64, 1.0, 2.5];
    let heavy = weights.binary_search_by(|w| w.total_cmp(&2.0));
    println!("{:?} {:?} {:?} {:?}", found, by_key, reversed, heavy);

    // A queue rotated either way.
    let mut q = VecDeque::from(vec![1, 2, 3, 4, 5]);
    q.rotate_left(2);
    println!("{:?}", q);
    q.rotate_right(3);
    println!("{:?}", q);
    let mut r = vec!['a', 'b', 'c'];
    r.rotate_left(1);
    r.rotate_right(2);
    println!("{:?}", r);

    // Sets and maps from arrays, by `into()` and `from`.
    let tags: BTreeSet<&str> = ["b", "a", "b"].into();
    let seen: HashSet<i32> = [3, 3, 4].into();
    let ages: BTreeMap<&str, u32> = [("bo", 4), ("al", 9)].into();
    let index: HashMap<char, usize> = HashMap::from([('x', 0), ('y', 1)]);
    let queue: VecDeque<u8> = [1, 2].into();
    let list: Vec<i32> = [7, 8].into();
    println!("{:?} {} {:?} {} {:?} {:?}", tags, seen.len(), ages, index[&'y'], queue, list);

    // What a `Cell` or a `RefCell` holds, replaced or taken.
    let count = Cell::new(5);
    let old = count.replace(6);
    let taken = count.take();
    println!("{} {} {}", old, taken, count.get());
    let label = Cell::new(Some('x'));
    println!("{:?} {:?}", label.take(), label.get());
    let log = RefCell::new(vec![1]);
    let before = log.replace(vec![2, 3]);
    log.borrow_mut().push(4);
    let after = log.take();
    println!("{:?} {:?} {:?}", before, after, log.borrow());
    let name = RefCell::new(String::from("old"));
    let previous = name.replace_with(|s| format!("{s}er"));
    println!("{} {}", previous, name.borrow());
}

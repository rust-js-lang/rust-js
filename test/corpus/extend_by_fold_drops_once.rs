// `extend` by `for_each`, a string's, a hash set's or map's, a B-tree's
// and a list's, reaches the crate's own `fold`, which takes the iterator
// and drops it, so nothing drops it again after; a `Vec`'s, a deque's and
// a heap's step it by `next` and drop it as they end (#1509).
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

struct Up {
    n: u32,
}

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("fold");
        (self.n + 1..=3).fold(init, |acc, x| f(acc, x))
    }
}

impl Drop for Up {
    fn drop(&mut self) {
        println!("drop Up {}", self.n);
    }
}

fn up() -> Up {
    Up { n: 0 }
}

struct Letters {
    n: u32,
}

impl Iterator for Letters {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.n < 3 {
            self.n += 1;
            char::from_digit(self.n, 10)
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, char) -> B>(self, init: B, mut f: F) -> B {
        println!("letters fold");
        let mut acc = init;
        for c in ['x', 'y'] {
            acc = f(acc, c);
        }
        acc
    }
}

impl Drop for Letters {
    fn drop(&mut self) {
        println!("drop Letters {}", self.n);
    }
}

fn main() {
    let mut s = String::new();
    s.extend(Letters { n: 0 });
    println!("letters {s}");
    let mut set3 = HashSet::new();
    set3.extend(up().skip(1));
    println!("skip set {}", set3.len());
    let mut set4 = BTreeSet::new();
    set4.extend(up().map(|x| x * 2));
    println!("map tree {set4:?}");
    let mut set = HashSet::new();
    set.extend(up());
    println!("extend set {}", set.len());
    let mut text = String::new();
    text.extend(up().map(|x| char::from_digit(x, 10).unwrap()));
    println!("extend string {text}");
    let mut tree = BTreeSet::new();
    tree.extend(up());
    println!("extend tree {tree:?}");
    let mut map = HashMap::new();
    map.extend(up().map(|x| (x, x)));
    println!("extend map {}", map.len());
    let mut sorted = BTreeMap::new();
    sorted.extend(up().map(|x| (x, x)));
    println!("extend sorted {sorted:?}");
    let mut list = LinkedList::new();
    list.extend(up());
    println!("extend list {list:?}");
    let mut deque = VecDeque::new();
    deque.extend(up());
    println!("extend deque {deque:?}");
    let mut heap = BinaryHeap::new();
    heap.extend(up());
    println!("extend heap {}", heap.len());
    let mut vec = vec![0];
    vec.extend(up());
    println!("extend vec {vec:?}");
    let set2: HashSet<u32> = up().collect();
    println!("collect set {}", set2.len());
    let tree2: BTreeSet<u32> = up().collect();
    println!("collect tree {}", tree2.len());
}

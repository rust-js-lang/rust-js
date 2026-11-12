//@ library-differs
// A `Pin` compares, orders and hashes its target, std's `Ptr::Target`: of
// the crate's own smart pointer, whose own `==`, `cmp` and `Hash` read a
// tag its target hasn't, what its `deref()` gives, so two pins of one key
// and two tags are equal and one key of a set (#1446, #1435); of std's
// pointers, a `Box`, a reference, an `Rc`, a `String`, a `Vec`, the pointer,
// which is its target in JS.
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
thread_local! { static HASHED: Cell<u32> = const { Cell::new(0) }; }
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Key(u32);
impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASHED.set(HASHED.get() + 1);
        self.0.hash(state);
    }
}
/// A pointer whose own `Hash`, `Eq` and `Ord` read its tag first.
#[derive(Debug)]
struct Tagged {
    tag: u32,
    key: Box<Key>,
}
impl PartialEq for Tagged {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.key == other.key
    }
}
impl Eq for Tagged {}
impl Hash for Tagged {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
        self.key.hash(state);
    }
}
impl PartialOrd for Tagged {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Tagged {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.tag, &self.key).cmp(&(other.tag, &other.key))
    }
}
impl Deref for Tagged {
    type Target = Key;
    fn deref(&self) -> &Key {
        &self.key
    }
}
fn pinned(key: u32, tag: u32) -> Pin<Tagged> {
    Pin::new(Tagged {
        tag,
        key: Box::new(Key(key)),
    })
}
fn main() {
    let (a, b, c) = (pinned(7, 1), pinned(7, 2), pinned(3, 9));
    println!("{} {} {} {}", a == b, a != b, a != c, c < a);
    println!("{:?} {:?} {:?}", a.cmp(&b), c.cmp(&a), a.partial_cmp(&c));
    println!("{}", std::cmp::max(c, pinned(1, 100)) == pinned(3, 0));
    let set: HashSet<Pin<Tagged>> = [pinned(1, 10), pinned(1, 20), pinned(2, 10)].into_iter().collect();
    println!(
        "{} {} {} hashed {}",
        set.len(),
        set.contains(&pinned(1, 99)),
        set.contains(&pinned(3, 10)),
        HASHED.get()
    );
    // Held in its order, not the table's, which std's `RandomState` makes
    // differ from run to run, as its drops would be.
    let mut kept: Vec<Pin<Tagged>> = set.into_iter().collect();
    kept.sort();
    println!("{}", kept.len());
    let mut map: HashMap<Pin<Tagged>, &str> = HashMap::new();
    map.insert(pinned(5, 1), "a");
    map.insert(pinned(5, 2), "b");
    println!("{} {:?}", map.len(), map.get(&pinned(5, 3)));
    let tree: BTreeSet<Pin<Tagged>> = [pinned(4, 1), pinned(2, 9), pinned(4, 5)].into_iter().collect();
    println!("{} {}", tree.len(), tree.first() == Some(&pinned(2, 0)));
    // std's own pointers compare their targets too.
    let (x, y) = (Box::pin(Key(1)), Box::pin(Key(1)));
    let (k1, k2) = (Key(2), Key(2));
    let (r1, r2) = (Pin::new(&k1), Pin::new(&k2));
    let (c1, c2) = (Pin::new(Rc::new(Key(3))), Pin::new(Rc::new(Key(4))));
    println!("{} {} {} {:?} {}", x == y, r1 == r2, c1 == c2, c1.cmp(&c2), r1 <= r2);
    let boxes: HashSet<Pin<Box<Key>>> = [Box::pin(Key(1)), Box::pin(Key(1))].into_iter().collect();
    println!("{}", boxes.len());
    let texts: HashSet<Pin<String>> = [Pin::new(String::from("a")), Pin::new(String::from("a"))]
        .into_iter()
        .collect();
    let lists = (Pin::new(vec![Key(1)]), Pin::new(vec![Key(1)]));
    println!("{} {} {}", texts.len(), lists.0 == lists.1, lists.0 < lists.1);
}

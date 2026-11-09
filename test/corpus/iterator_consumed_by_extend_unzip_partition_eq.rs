// An iterator with a destructor that std's `extend`, `unzip`, `partition`,
// `eq` and `ne` take by value is dropped once, as each ends: every
// collection's `extend` (a map's and a set's never dropped it, #1122; a
// `Vec`'s, a `String`'s and a deque's were refused, #1120), `unzip` and
// `partition` (refused, #1120), `eq` and `ne` (dropped twice, #1123); of
// items with a destructor too, and as a panic in `partition`'s function
// unwinds, what's left and the iterator dropped then, and in what a
// `String`'s `extend` steps, its source dropped as it unwinds.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

struct Loud<I>(I, &'static str);

impl<I: Iterator> Iterator for Loud<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

impl<I> Drop for Loud<I> {
    fn drop(&mut self) {
        println!("drop {}", self.1);
    }
}

fn loud<T>(v: Vec<T>, name: &'static str) -> Loud<std::vec::IntoIter<T>> {
    Loud(v.into_iter(), name)
}

#[derive(Debug, PartialEq)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop D{}", self.0);
    }
}

fn main() {
    let mut v = vec![0u32];
    v.extend(loud(vec![1, 2], "vec"));
    println!("{v:?}");
    let mut s = String::from("a");
    s.extend(loud(vec!["b", "c"], "string"));
    println!("{s}");
    let mut d = VecDeque::from([0u32]);
    d.extend(loud(vec![1], "deque"));
    println!("{d:?}");
    let mut m = BTreeMap::from([(1u32, 1u32)]);
    m.extend(loud(vec![(1, 10), (2, 20)], "btreemap"));
    println!("{m:?}");
    let mut t = BTreeSet::new();
    t.extend(loud(vec![3u32, 3], "btreeset"));
    println!("{t:?}");
    let mut h = HashMap::new();
    h.extend(loud(vec![(1u32, 'x')], "hashmap"));
    println!("{}", h.len());
    let mut hs = HashSet::new();
    hs.extend(loud(vec![1u32, 1], "hashset"));
    println!("{}", hs.len());

    let (a, b): (Vec<u32>, Vec<char>) = loud(vec![(1, 'p'), (2, 'q')], "unzip").unzip();
    println!("{a:?} {b:?}");
    let (even, odd): (Vec<u32>, Vec<u32>) = loud(vec![1, 2, 3], "partition").partition(|n| n % 2 == 0);
    println!("{even:?} {odd:?}");
    println!("{}", loud(vec![1u32, 2], "eq").eq(vec![1, 2]));
    println!("{}", loud(vec![1u32, 2], "ne").ne([1, 3]));

    let mut ds = vec![D(0)];
    ds.extend(loud(vec![D(1), D(2)], "items"));
    println!("{}", ds.len());
    let (big, small): (Vec<D>, Vec<D>) = loud(vec![D(5), D(6)], "items partition").partition(|d| d.0 > 5);
    println!("{big:?} {small:?}");
    let mut lent = vec![D(10), D(11), D(12)].into_iter();
    let (high, low): (Vec<D>, Vec<D>) = lent.by_ref().partition(|d| d.0 > 10);
    println!("{high:?} {low:?} {}", lent.len());
    let (ds, ns): (Vec<D>, Vec<u32>) = loud(vec![(D(3), 4), (D(4), 5)], "items unzip").unzip();
    println!("{ds:?} {ns:?}");
    let caught = std::panic::catch_unwind(|| {
        loud(vec![D(7), D(8), D(9)], "panicking").partition::<Vec<D>, _>(|d| {
            if d.0 == 8 {
                panic!("at 8");
            }
            true
        })
    });
    println!("{}", caught.is_err());
    let mut text = String::from(">");
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        text.extend(loud(vec!['a', 'b', 'c'], "text").map(|c| if c == 'c' { panic!("at c") } else { c }));
    }));
    println!("{}", caught.is_err());
}

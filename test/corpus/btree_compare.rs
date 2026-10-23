// B-trees compare as their items in order, as sequences do, a map's its
// entries: by `Ord`, the keys' own too, and by `PartialOrd` of `f64`s.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq, Eq)]
struct Rev(u8);
impl Ord for Rev {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.cmp(&self.0)
    }
}
impl PartialOrd for Rev {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn main() {
    let a = BTreeSet::from([1, 2]);
    let b = BTreeSet::from([1, 3]);
    println!("{} {:?} {:?}", a < b, a.cmp(&b), b.partial_cmp(&a));
    let m = BTreeMap::from([(1, "x")]);
    let n = BTreeMap::from([(1, "y"), (0, "z")]);
    println!("{:?} {}", m.cmp(&n), m > n);
    let mut v = vec![BTreeSet::from([2]), BTreeSet::from([1, 5]), BTreeSet::new()];
    v.sort();
    println!("{:?} {:?}", v, v.iter().max());
    others();
}

fn others() {
    let a = BTreeSet::from([Rev(1), Rev(5)]);
    let b = BTreeSet::from([Rev(1), Rev(4)]);
    println!("{:?} {}", a.cmp(&b), a < b);
    let x = BTreeMap::from([(1, f64::NAN)]);
    let y = BTreeMap::from([(1, 2.0)]);
    let z = BTreeMap::from([(0, 9.0)]);
    println!("{:?} {:?} {}", x.partial_cmp(&y), y.partial_cmp(&z), y > z);
    let nested = BTreeSet::from([BTreeSet::from([2, 1]), BTreeSet::from([1])]);
    println!("{:?}", nested);
}

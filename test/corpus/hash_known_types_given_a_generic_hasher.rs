//@ native: wasm32
//@ library-differs
// std's types hashed into a hasher generic code is given, `state: &mut H`,
// and std's collections extended in generic code: each a call of std's own
// impl, whose `&mut` is what it's given, no dictionary's box (ADR 0463).
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Many;

impl Hash for Many {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (1u8, 'c').hash(state);
        [1u16, 2, 3].hash(state);
        Some(5i64).hash(state);
        "text".hash(state);
        String::from("owned").hash(state);
        ().hash(state);
        Ordering::Less.hash(state);
        Box::new(9u8).hash(state);
        Rc::new(3u8).hash(state);
        vec![1u8, 2].hash(state);
        (&4u32).hash(state);
        true.hash(state);
        Ok::<u8, char>(2).hash(state);
    }
}

fn fill<E: Extend<u8>>(into: &mut E) {
    into.extend([1, 2, 3]);
}

fn grown(v: &mut Vec<u8>, d: &mut VecDeque<u8>, s: &mut BTreeSet<u8>, m: &mut HashMap<u8, u8>) {
    fill(v);
    fill(d);
    fill(s);
    v.extend([4, 5]);
    d.extend(vec![6]);
    s.extend(&[7, 8]);
    m.extend([(1, 2)]);
}

fn main() {
    let mut h = DefaultHasher::new();
    Many.hash(&mut h);
    println!("{}", h.finish());
    let (mut v, mut d, mut s, mut m) = (Vec::new(), VecDeque::new(), BTreeSet::new(), HashMap::new());
    grown(&mut v, &mut d, &mut s, &mut m);
    println!("{v:?} {d:?} {s:?} {:?}", m.get(&1));
}

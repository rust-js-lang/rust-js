// A set's or a map's key that holds a type equal only to itself, as
// React's fibers are (ADR 0864): a derived struct, a tuple, an `Option`, a
// `Vec` and an enum of one, each found by its value where the rest of it
// is concerned and by the fiber itself where the fiber is, as its derived
// `Hash` and `Eq` find it. Two fibers alike are two keys, one fiber twice
// is one key, and a cycle of fibers is never walked (ADR 1015, amended).
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Fiber {
    tag: u32,
    alternate: RefCell<Option<Rc<Fiber>>>,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

fn fiber(tag: u32) -> Rc<Fiber> {
    Rc::new(Fiber {
        tag,
        alternate: RefCell::new(None),
    })
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    fiber: Rc<Fiber>,
    lane: u32,
}

#[derive(PartialEq, Eq, Hash)]
struct Outer {
    key: Key,
    name: &'static str,
}

#[derive(PartialEq, Eq, Hash)]
enum Target {
    Fiber(Rc<Fiber>),
    Lane(u32),
    Both { fiber: Rc<Fiber>, lane: u32 },
}

fn key(fiber: &Rc<Fiber>, lane: u32) -> Key {
    Key {
        fiber: Rc::clone(fiber),
        lane,
    }
}

fn distinct<K: Hash + Eq>(keys: Vec<K>) -> usize {
    keys.into_iter().collect::<HashSet<K>>().len()
}

fn main() {
    let a = fiber(1);
    let b = fiber(1);
    // A cycle, which a key walking its fields wouldn't leave.
    *a.alternate.borrow_mut() = Some(Rc::clone(&b));
    *b.alternate.borrow_mut() = Some(Rc::clone(&a));

    let mut m = HashMap::new();
    m.insert(key(&a, 1), "a");
    m.insert(key(&b, 1), "b");
    m.insert(key(&a, 2), "a2");
    println!("{} {:?} {:?}", m.len(), m.get(&key(&a, 1)), m.get(&key(&b, 1)));
    *m.entry(key(&a, 1)).or_insert("none") = "a again";
    let removed = m.remove(&key(&b, 1));
    println!("{} {:?} {:?}", m.len(), m.get(&key(&a, 1)), removed);
    println!("{} {}", m.contains_key(&key(&b, 1)), m.contains_key(&key(&a, 2)));

    let mut set = HashSet::new();
    println!(
        "{} {} {}",
        set.insert(key(&a, 1)),
        set.insert(key(&b, 1)),
        set.insert(key(&a, 1))
    );
    println!("{}", set.len());

    let mut pairs: HashMap<(Rc<Fiber>, u32), u32> = HashMap::new();
    *pairs.entry((Rc::clone(&a), 1)).or_default() += 1;
    *pairs.entry((Rc::clone(&b), 1)).or_default() += 10;
    *pairs.entry((Rc::clone(&a), 1)).or_default() += 100;
    println!("{} {:?}", pairs.len(), pairs.get(&(Rc::clone(&a), 1)));

    let instances: HashSet<Option<Rc<Fiber>>> = [Some(Rc::clone(&a)), Some(Rc::clone(&b)), None, Some(Rc::clone(&a))]
        .into_iter()
        .collect();
    println!("{} {}", instances.len(), instances.contains(&None));

    let paths: HashSet<Vec<Rc<Fiber>>> = [
        vec![Rc::clone(&a), Rc::clone(&b)],
        vec![Rc::clone(&b), Rc::clone(&a)],
        vec![Rc::clone(&a), Rc::clone(&b)],
    ]
    .into_iter()
    .collect();
    println!("{}", paths.len());

    let outers: HashSet<Outer> = [
        Outer {
            key: key(&a, 1),
            name: "x",
        },
        Outer {
            key: key(&b, 1),
            name: "x",
        },
        Outer {
            key: key(&a, 1),
            name: "x",
        },
    ]
    .into_iter()
    .collect();
    println!("{}", outers.len());

    let maybe: HashSet<Option<Key>> = [Some(key(&a, 1)), None, Some(key(&b, 1)), None, Some(key(&a, 1))]
        .into_iter()
        .collect();
    println!("{} {}", maybe.len(), maybe.contains(&None));

    let targets: HashSet<Target> = [
        Target::Fiber(Rc::clone(&a)),
        Target::Fiber(Rc::clone(&b)),
        Target::Lane(1),
        Target::Lane(1),
        Target::Both {
            fiber: Rc::clone(&a),
            lane: 1,
        },
        Target::Both {
            fiber: Rc::clone(&b),
            lane: 1,
        },
        Target::Both {
            fiber: Rc::clone(&a),
            lane: 1,
        },
    ]
    .into_iter()
    .collect();
    println!("{}", targets.len());

    // Generic code given such a key makes the same map.
    println!(
        "{} {}",
        distinct(vec![key(&a, 1), key(&b, 1), key(&a, 1)]),
        distinct(vec![(1, 2), (1, 2)])
    );
    let mut cloned = m.clone();
    cloned.insert(key(&b, 3), "b3");
    println!("{} {} {}", m.len(), cloned.len(), a.tag + b.tag);
    *a.alternate.borrow_mut() = None;
}

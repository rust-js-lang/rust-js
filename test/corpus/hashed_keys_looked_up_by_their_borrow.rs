//@ library-differs
// A hash table whose keys are found by their own `Hash` and `Eq`, looked up
// by what the key's `Borrow` gives: an `Rc` nothing counts, a `Box` and a
// `Vec`, each in JS the very value it borrows as, so the lookup hashes that
// value by the key's `hash` and compares it by its `eq`, as std's does,
// `Borrow` keeping both the same, once per lookup. Each was refused
// (#1288).
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
thread_local! { static HASHED: Cell<u32> = const { Cell::new(0) }; }
#[derive(PartialEq, Eq, Debug, Clone)]
struct Point(i32, i32);
impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASHED.set(HASHED.get() + 1);
        self.0.hash(state);
        self.1.hash(state);
    }
}
fn main() {
    let s: HashSet<Rc<u32>> = [Rc::new(1), Rc::new(2)].into_iter().collect();
    println!("{} {}", s.contains(&1), s.contains(&3));
    let ps: HashSet<Rc<Point>> = [Rc::new(Point(1, 2))].into_iter().collect();
    let before = HASHED.get();
    println!(
        "{} {} hashed {}",
        ps.contains(&Point(1, 2)),
        ps.contains(&Point(2, 1)),
        HASHED.get() - before
    );
    let bs: HashMap<Box<Point>, &str> = [(Box::new(Point(3, 4)), "b")].into_iter().collect();
    println!("{:?} {:?}", bs.get(&Point(3, 4)), bs.get(&Point(0, 0)));
    let vs: HashSet<Vec<Point>> = [vec![Point(1, 1), Point(2, 2)]].into_iter().collect();
    println!(
        "{} {}",
        vs.contains(&[Point(1, 1), Point(2, 2)][..]),
        vs.contains(&[Point(1, 1)][..])
    );
}

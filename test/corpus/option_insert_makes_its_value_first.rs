// `Option::insert` makes its value before it drops the one it replaces, as
// Rust evaluates the argument before the call: in a function where nothing
// else has a cleanup, so no flush of another's orders it (#765). And the
// writes beside it that replace a value with a destructor, each the new
// value made first: `replace`, `get_or_insert`, `mem::replace`, an
// assignment through a `&mut`, of an `Option` too, a map's `insert`, an
// element's and a field's.
use std::collections::{BTreeMap, HashMap};

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn made(n: u32) -> Noisy {
    println!("made {n}");
    Noisy(n)
}

fn insert(slot: &mut Option<Noisy>) -> u32 {
    slot.insert(made(5)).0
}

fn insert_boxed(slot: &mut Option<Option<Noisy>>) -> bool {
    slot.insert(Some(made(17))).is_some()
}

fn replace(slot: &mut Option<Noisy>) -> Option<u32> {
    slot.replace(made(6)).map(|n| n.0)
}

fn mem_replace(slot: &mut Noisy) -> u32 {
    std::mem::replace(slot, made(7)).0
}

fn assign(slot: &mut Noisy) {
    *slot = made(8);
}

fn assign_some(slot: &mut Option<Noisy>) {
    *slot = Some(made(9));
}

fn map_insert(m: &mut HashMap<u32, Noisy>) -> Option<u32> {
    m.insert(1, made(10)).map(|n| n.0)
}

fn tree_insert(m: &mut BTreeMap<u32, Noisy>) -> Option<u32> {
    m.insert(1, made(11)).map(|n| n.0)
}

fn element(v: &mut [Noisy]) {
    v[0] = made(12);
}

fn get_or_insert(slot: &mut Option<Noisy>) -> u32 {
    slot.get_or_insert(made(13)).0
}

fn field(p: &mut (Noisy, u32)) {
    p.0 = made(14);
}

fn main() {
    let mut slot = Some(Noisy(1));
    println!("insert {}", insert(&mut slot));
    let mut boxed = Some(Some(Noisy(18)));
    println!("insert boxed {}", insert_boxed(&mut boxed));
    println!("replace {:?}", replace(&mut slot));
    let mut n = Noisy(2);
    println!("mem_replace {}", mem_replace(&mut n));
    assign(&mut n);
    println!("assigned {}", n.0);
    assign_some(&mut slot);
    let mut m = HashMap::new();
    m.insert(1, Noisy(3));
    println!("map {:?}", map_insert(&mut m));
    let mut t = BTreeMap::new();
    t.insert(1, Noisy(4));
    println!("tree {:?}", tree_insert(&mut t));
    let mut v = vec![Noisy(15)];
    element(&mut v);
    println!("get_or_insert {}", get_or_insert(&mut slot));
    let mut p = (Noisy(16), 0);
    field(&mut p);
    println!("end");
}

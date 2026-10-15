// A map whose values could look like `None`, `()` or an `Option`, gives
// `Some` of one boxed where it does (ADR 0051): `get`, `insert` and
// `remove` tell a value there from none (ADR 0403).
use std::collections::{BTreeMap, HashMap};
fn main() {
    let mut units: HashMap<&str, ()> = HashMap::new();
    println!("{:?}", units.insert("a", ()));
    println!("{:?}", units.insert("a", ()));
    println!("{:?} {:?}", units.get("a"), units.get("b"));
    println!("{:?} {:?}", units.remove("a"), units.remove("a"));
    let mut opts: BTreeMap<u8, Option<u32>> = BTreeMap::new();
    println!("{:?}", opts.insert(1, None));
    println!("{:?}", opts.insert(1, Some(5)));
    println!("{:?} {:?}", opts.get(&1), opts.get(&2));
    opts.insert(2, None);
    println!("{:?}", opts.get(&2));
    println!("{:?}", opts.remove(&2));
    if let Some(v) = opts.get_mut(&1) { *v = None; }
    println!("{:?}", opts.get(&1));
    if let Some(v) = units.get_mut("zz") { *v = (); }
    units.insert("q", ());
    if let Some(v) = units.get_mut("q") { *v = (); println!("got q"); }
}

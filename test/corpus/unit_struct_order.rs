// A struct of no fields orders as its derive has it: its one value is
// `Equal` to itself. As a map's key, compared, and as a field of a struct
// ordered field by field.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Unit;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tagged {
    tag: Unit,
    n: u32,
}

fn main() {
    let mut map = BTreeMap::new();
    map.insert(Unit, 1);
    map.insert(Unit, 2);
    println!("{} {:?}", map.len(), map.get(&Unit));

    let set: BTreeSet<Unit> = [Unit, Unit, Unit].into_iter().collect();
    println!("{set:?}");

    println!("{:?} {} {}", Unit.cmp(&Unit), Unit < Unit, Unit <= Unit);
    println!("{:?}", Unit.partial_cmp(&Unit) == Some(Ordering::Equal));

    let mut tagged = vec![Tagged { tag: Unit, n: 3 }, Tagged { tag: Unit, n: 1 }];
    tagged.sort();
    println!("{tagged:?} {:?}", tagged.iter().max());
}

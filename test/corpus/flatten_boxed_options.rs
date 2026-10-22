use std::collections::BTreeMap;
use std::iter;

fn main() {
    let v: Vec<Option<Option<i32>>> = vec![Some(None), None, Some(Some(1))];
    for x in v.iter().flatten() {
        println!("{:?} {}", x, x.is_none());
    }
    println!("{:?}", v.iter().copied().flatten().next());
    let w: Vec<Option<i32>> = v.into_iter().flatten().collect();
    println!("{:?}", w);

    let units: Vec<Option<Option<()>>> = vec![Some(None), Some(Some(()))];
    println!("{:?}", units.into_iter().flatten().collect::<Vec<_>>());

    let mut m = BTreeMap::new();
    m.insert(1, Some(None::<i32>));
    m.insert(2, Some(Some(2)));
    println!("{:?}", m.values().flatten().collect::<Vec<_>>());
    println!("{:?}", iter::once(Some(None::<u8>)).flatten().collect::<Vec<_>>());

    let mut out: Vec<Option<i32>> = vec![Some(0)];
    out.extend([Some(None), Some(Some(3))].into_iter().flatten());
    println!("{:?}", out);
}

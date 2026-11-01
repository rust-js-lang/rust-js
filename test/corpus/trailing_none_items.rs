// A `None` last among what a std call gives JS's own method, `push`,
// `splice`, `unshift`, `concat`: an item, never left out, as JS's method
// counts what it's given (ADRs 0313, 0330).
use std::collections::{BTreeSet, HashMap, VecDeque};

fn shown(items: &[Option<u32>]) -> String {
    format!("{} {:?}", items.len(), items)
}

fn main() {
    let mut v: Vec<Option<u32>> = vec![Some(1)];
    v.push(None);
    v.insert(1, None);
    v.extend([Some(2), None]);
    v.extend(vec![None, None]);
    v.extend_from_slice(&[Some(3), None]);
    v.splice(1..2, [Some(4), None]);
    v.resize(12, None);
    v.append(&mut vec![Some(5), None]);
    println!("{}", shown(&v));
    let mut d: VecDeque<Option<u32>> = VecDeque::new();
    d.push_front(None);
    d.push_front(Some(6));
    d.push_back(None);
    d.extend([None, Some(7), None]);
    d.insert(1, None);
    println!("{} {:?}", d.len(), d);
    let joined: Vec<Option<u32>> = [vec![Some(8)], vec![None]].concat();
    let chained: Vec<Option<u32>> = v.iter().copied().chain([None]).collect();
    println!("{} {}", shown(&joined), chained.len());
    let mut m: HashMap<u32, Option<u32>> = HashMap::new();
    m.insert(1, None);
    m.insert(2, Some(9));
    let mut keys: Vec<_> = m.iter().map(|(k, v)| (*k, *v)).collect();
    keys.sort();
    println!("{} {:?}", m.len(), keys);
    let s: BTreeSet<Option<u32>> = [None, Some(1), None].into_iter().collect();
    println!("{:?} {:?}", s, [Some(2), None].iter().max());
    let mut w = vec![None; 3];
    w[1] = Some(10);
    println!("{}", shown(&w));
}

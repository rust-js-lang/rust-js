//@ native: wasm32
//@ library-differs
// A `PhantomData` hashes nothing, as std's impl writes nothing (ADR 0463),
// and is equal to every other, and `Equal` (ADR 0121): litemap's map
// derives `Hash` and `Ord` of a generic store and two markers of unsized
// `K` and `V`, so it hashes and orders as its store does, through
// `Hash::hash`, and is a key of a set, a map and a B-tree (#514).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct LiteMap<K: ?Sized, V: ?Sized, S> {
    values: S,
    _key_type: PhantomData<K>,
    _value_type: PhantomData<V>,
}

impl<K: ?Sized, V: ?Sized, S> LiteMap<K, V, S> {
    fn from_store(values: S) -> Self {
        LiteMap {
            values,
            _key_type: PhantomData,
            _value_type: PhantomData,
        }
    }
}

type Map = LiteMap<str, [u8], Vec<(u8, u8)>>;

#[derive(Hash)]
struct Unit;

fn digest<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    Hash::hash(value, &mut hasher);
    hasher.finish()
}

fn main() {
    let store = vec![(1, 2), (3, 4)];
    let map: Map = LiteMap::from_store(store.clone());
    println!("{} {}", digest(&map), digest(&map) == digest(&store));
    println!("{:?}", map.cmp(&LiteMap::from_store(vec![(1, 3)])));
    println!("{}", map < LiteMap::from_store(vec![(0, 9)]));
    let (key, other) = (PhantomData::<str>, PhantomData::<str>);
    println!("{:?} {}", key.cmp(&other), key <= other);
    println!("{}", digest(&PhantomData::<str>) == digest(&()));
    println!("{} {}", digest(&Unit), digest(&(PhantomData::<u8>, 5u8)));
    println!("{}", digest(&[PhantomData::<u8>; 3]));

    let sorted: BTreeSet<Map> = [vec![(2, 0)], store.clone(), vec![]]
        .into_iter()
        .map(LiteMap::from_store)
        .collect();
    println!("{:?}", sorted.iter().map(|m| &m.values).collect::<Vec<_>>());

    let mut seen: HashSet<Map> = HashSet::new();
    println!("{}", seen.insert(map));
    println!("{}", seen.insert(LiteMap::from_store(store.clone())));
    println!("{}", seen.insert(LiteMap::from_store(vec![(1, 2)])));
    let mut sizes: HashMap<Map, usize> = HashMap::new();
    for values in [store.clone(), vec![], store] {
        let n = values.len();
        *sizes.entry(LiteMap::from_store(values)).or_default() += n;
    }
    let mut counts: Vec<_> = sizes.into_iter().map(|(m, n)| (m.values, n)).collect();
    counts.sort();
    println!("{counts:?}");
}

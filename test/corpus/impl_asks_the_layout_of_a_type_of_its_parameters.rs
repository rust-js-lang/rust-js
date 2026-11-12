//@ library-differs
// An impl whose method asks the layout of a type built of the impl's
// parameters, quickcheck 0.9's `BTreeMap<K, V>` and `HashMap<K, V, S>`
// made of a `Vec<(K, V)>` (#1420): the impl's dictionary is given it.
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash};

pub trait Arbitrary: Clone + 'static {
    fn arbitrary(n: u32) -> Self;
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(std::iter::empty())
    }
}

impl Arbitrary for u32 {
    fn arbitrary(n: u32) -> u32 {
        n
    }
}

impl<A: Arbitrary> Arbitrary for Vec<A> {
    fn arbitrary(n: u32) -> Vec<A> {
        (0..n).map(A::arbitrary).collect()
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Vec<A>>> {
        let mut v = self.clone();
        v.pop();
        Box::new(std::iter::once(v))
    }
}

impl<A: Arbitrary, B: Arbitrary> Arbitrary for (A, B) {
    fn arbitrary(n: u32) -> (A, B) {
        (A::arbitrary(n), B::arbitrary(n + 1))
    }
}

impl<K: Arbitrary + Ord, V: Arbitrary> Arbitrary for BTreeMap<K, V> {
    fn arbitrary(n: u32) -> BTreeMap<K, V> {
        let vec: Vec<(K, V)> = Arbitrary::arbitrary(n);
        vec.into_iter().collect()
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = BTreeMap<K, V>>> {
        let vec: Vec<(K, V)> = self.clone().into_iter().collect();
        Box::new(vec.shrink().map(|v| v.into_iter().collect::<BTreeMap<K, V>>()))
    }
}

impl<K: Arbitrary + Eq + Hash, V: Arbitrary, S: BuildHasher + Default + Clone + 'static> Arbitrary
    for HashMap<K, V, S>
{
    fn arbitrary(n: u32) -> Self {
        let vec: Vec<(K, V)> = Arbitrary::arbitrary(n);
        vec.into_iter().collect()
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let vec: Vec<(K, V)> = self.clone().into_iter().collect();
        Box::new(vec.shrink().map(|v| v.into_iter().collect::<Self>()))
    }
}

pub fn made<A: Arbitrary>(n: u32) -> A {
    A::arbitrary(n)
}


fn main() {
    let m: BTreeMap<u32, u32> = made(3);
    println!("{m:?}");
    println!("{:?}", m.shrink().collect::<Vec<_>>());
    let h: HashMap<u32, (u32, u32), BuildHasherDefault<DefaultHasher>> = made(4);
    let mut pairs: Vec<_> = h.into_iter().collect();
    pairs.sort();
    println!("{pairs:?}");
    let nested: BTreeMap<(u32, u32), Vec<u32>> = made(2);
    println!("{nested:?} {}", nested.shrink().count());
}

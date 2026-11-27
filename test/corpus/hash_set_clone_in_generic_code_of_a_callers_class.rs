//@ library-differs
// A `HashSet`'s or a `HashMap`'s clone in generic code, of a class its
// caller gives (ADR 0466): of numbers or strings, a `Set` or a `Map`, which
// finds them by value; of keys whose own `Hash` prints, a hashed one, whose
// clone asks no key's `hash` (#2543); of a tuple of both, one composed of
// its parts'. Each is cloned as std clones it, the class asked how, where
// a `Set` of numbers was given `ofHashed`, which it hasn't, and panicked
// "TSet.ofHashed is not a function" (itertools' `correct_permutations`
// through quickcheck's `Arbitrary` of a `HashSet<i32>`, #2625).
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Eq)]
struct Key(u32);

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        println!("hash {}", self.0);
        self.0.hash(state);
    }
}

fn cloned<T: Clone + Hash + Eq>(set: &HashSet<T>) -> HashSet<T> {
    set.clone()
}

fn cloned_map<K: Clone + Hash + Eq, V: Clone>(map: &HashMap<K, V>) -> HashMap<K, V> {
    map.clone()
}

fn main() {
    let numbers: HashSet<i32> = [1, 2, 3].into_iter().collect();
    let copy = cloned(&numbers);
    println!("numbers {} {}", copy.len(), copy.contains(&2));
    let keys: HashSet<Key> = [Key(3), Key(4)].into_iter().collect();
    println!("cloning keys");
    let copy = cloned(&keys);
    println!("keys {} {}", copy.len(), copy.contains(&Key(3)));
    let words: HashMap<&str, u8> = [("a", 1), ("b", 2)].into_iter().collect();
    let copy = cloned_map(&words);
    println!("words {} {:?}", copy.len(), copy.get("b"));
    let keyed: HashMap<Key, &str> = [(Key(6), "f")].into_iter().collect();
    println!("cloning keyed");
    let copy = cloned_map(&keyed);
    println!("keyed {} {:?}", copy.len(), copy.get(&Key(6)));
    let pairs: HashSet<(i32, Key)> = [(1, Key(5))].into_iter().collect();
    println!("cloning pairs");
    let copy = cloned(&pairs);
    println!("pairs {} {}", copy.len(), copy.contains(&(1, Key(5))));
    let plain: HashSet<(i32, u8)> = [(1, 2), (3, 4)].into_iter().collect();
    let copy = cloned(&plain);
    println!("plain pairs {} {}", copy.len(), copy.contains(&(3, 4)));
}

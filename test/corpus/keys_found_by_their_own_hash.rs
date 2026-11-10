//@ library-differs
// A hash map's and a hash set's keys whose `Eq` and `Hash` are their own:
// a word equal to another whatever its case, which hashes its lowercase
// form, as a case-insensitive index does. Which keys a map holds is the
// impl's `==`, never the fields' values, so `Word("Apple")` finds the
// `Word("apple")` it holds, and the map keeps the key it was first given.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
struct Word(String);

impl PartialEq for Word {
    fn eq(&self, other: &Word) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for Word {}

impl Hash for Word {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state);
    }
}

// Equal by its `id` alone, its `name` hashed by no one: two ids' bytes
// differ, so they're never compared, as std's table never compares keys
// whose hashes differ.
#[derive(Debug, Clone)]
struct User {
    id: u32,
    name: String,
}

impl PartialEq for User {
    fn eq(&self, other: &User) -> bool {
        self.id == other.id
    }
}

impl Eq for User {}

impl Hash for User {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// Hashes less than `==` compares: every key hashes the same, so each search
// asks `eq` of the keys it holds.
#[derive(Debug, Clone, Copy)]
struct Parity(u32);

impl PartialEq for Parity {
    fn eq(&self, other: &Parity) -> bool {
        self.0 % 2 == other.0 % 2
    }
}

impl Eq for Parity {}

impl Hash for Parity {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

thread_local! {
    static PANIC_AT: Cell<u32> = const { Cell::new(0) };
}

// A key whose `hash` panics where it's told to: what a panicking search
// leaves.
#[derive(Debug, PartialEq, Eq)]
struct Fragile(u32);

impl Hash for Fragile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if PANIC_AT.with(Cell::get) == self.0 {
            panic!("hash of {}", self.0);
        }
        self.0.hash(state);
    }
}

// A derived `Eq` and `Hash` of a field whose are its own: found by the
// field's.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Tagged {
    word: Word,
    n: u32,
}

// Generic code's map, of the class its caller's keys are found by.
fn count<K: Hash + Eq + Clone>(items: &[K]) -> HashMap<K, usize> {
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item.clone()).or_insert(0) += 1;
    }
    counts
}

// A derived `Hash` of a field whose own `hash` panics, as std's of an
// `Option` of one is: each calls it.
#[derive(Debug, PartialEq, Eq, Hash)]
struct Held {
    fragile: Fragile,
}

fn sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

fn index(text: &str) -> HashMap<Word, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(Word(word.to_string())).or_insert(0) += 1;
    }
    counts
}

fn words(map: &HashMap<Word, usize>) -> Vec<(String, usize)> {
    sorted(map.iter().map(|(w, n)| (w.0.clone(), *n)).collect())
}

fn maps() {
    let mut counts = index("apple Banana APPLE banana cherry Apple");
    println!("{:?}", words(&counts));
    println!("{}", counts.len());
    println!("{:?}", counts.get(&Word("APPLE".into())));
    println!("{:?}", counts.get(&Word("durian".into())));
    println!("{}", counts.contains_key(&Word("CHERRY".into())));
    // `insert` of a key it holds keeps the key it holds, the value given.
    println!("{:?}", counts.insert(Word("BANANA".into()), 10));
    println!("{:?}", counts.get_key_value(&Word("banana".into())));
    if let Some(n) = counts.get_mut(&Word("Cherry".into())) {
        *n += 5;
    }
    println!("{:?}", counts.remove_entry(&Word("CHERRY".into())));
    println!("{:?}", counts.remove(&Word("cherry".into())));
    println!("{:?}", words(&counts));

    counts.retain(|w, _| w.0.starts_with('a'));
    println!("{:?}", words(&counts));
    counts.extend([(Word("Apple".into()), 1), (Word("Date".into()), 2), (Word("DATE".into()), 3)]);
    println!("{:?}", words(&counts));

    let copy = counts.clone();
    println!("{}", copy == counts);
    let mut other = HashMap::new();
    other.insert(Word("APPLE".into()), 1);
    other.insert(Word("date".into()), 3);
    println!("{}", other == counts);
    println!("{:?}", words(&other));

    let users: HashMap<User, &str> = [
        (User { id: 1, name: "ana".into() }, "admin"),
        (User { id: 2, name: "bo".into() }, "guest"),
        (User { id: 1, name: "ann".into() }, "owner"),
    ]
    .into_iter()
    .collect();
    let mut seen: Vec<(u32, String, &str)> = users.iter().map(|(u, r)| (u.id, u.name.clone(), *r)).collect();
    seen.sort();
    println!("{seen:?}");
    let probe = User { id: 2, name: "nobody".into() };
    println!("{:?}", users.get_key_value(&probe).map(|(u, r)| (&u.name, r)));

    let mut parities = HashMap::new();
    for n in 0..6 {
        *parities.entry(Parity(n)).or_insert(0) += n;
    }
    let mut kept: Vec<(u32, u32)> = parities.iter().map(|(p, n)| (p.0, *n)).collect();
    kept.sort();
    println!("{kept:?}");
    match parities.entry(Parity(7)) {
        std::collections::hash_map::Entry::Occupied(e) => println!("occupied {:?} {}", e.key(), e.get()),
        std::collections::hash_map::Entry::Vacant(_) => println!("vacant"),
    }
}

fn sets() {
    let mut seen: HashSet<Word> = HashSet::new();
    for word in "To be or not TO BE".split(' ') {
        println!("{word} {}", seen.insert(Word(word.into())));
    }
    let mut kept: Vec<String> = seen.iter().map(|w| w.0.clone()).collect();
    kept.sort();
    println!("{kept:?}");
    println!("{}", seen.contains(&Word("NOT".into())));
    println!("{:?}", seen.get(&Word("OR".into())));
    // `replace` puts the one given in the place of the one it held.
    println!("{:?}", seen.replace(Word("Be".into())));
    println!("{:?}", seen.get(&Word("be".into())));
    println!("{:?}", seen.take(&Word("Or".into())));
    println!("{}", seen.remove(&Word("NoT".into())));
    println!("{}", seen.len());

    let left: HashSet<Word> = ["a", "B", "c"].map(|s| Word(s.into())).into_iter().collect();
    let right: HashSet<Word> = ["A", "b", "D"].map(|s| Word(s.into())).into_iter().collect();
    let each = |words: Vec<&Word>| sorted(words.into_iter().map(|w| w.0.clone()).collect::<Vec<_>>());
    println!("{:?}", each(left.intersection(&right).collect()));
    println!("{:?}", each(left.union(&right).collect()));
    println!("{:?}", each(left.difference(&right).collect()));
    println!("{:?}", each(left.symmetric_difference(&right).collect()));
    let small: HashSet<Word> = [Word("C".into())].into_iter().collect();
    println!("{} {}", small.is_subset(&left), left.is_disjoint(&right));
    let upper: HashSet<Word> = ["A", "b", "C"].map(|s| Word(s.into())).into_iter().collect();
    println!("{}", left == upper);
    let mut ones: HashSet<Parity> = HashSet::new();
    ones.extend([Parity(1), Parity(3), Parity(4)]);
    let mut held: Vec<u32> = ones.iter().map(|p| p.0).collect();
    held.sort();
    println!("{held:?}");
}

fn holding() {
    let items = [Word("a".into()), Word("A".into()), Word("b".into())];
    println!("{:?}", words(&count(&items)));

    let mut pairs: HashMap<(Word, u32), &str> = HashMap::new();
    pairs.insert((Word("x".into()), 1), "one");
    println!("{:?}", pairs.get(&(Word("X".into()), 1)));
    println!("{:?}", pairs.get(&(Word("X".into()), 2)));

    let mut tagged: HashSet<Tagged> = HashSet::new();
    tagged.insert(Tagged { word: Word("Hi".into()), n: 1 });
    println!("{}", tagged.insert(Tagged { word: Word("hI".into()), n: 1 }));
    println!("{}", tagged.contains(&Tagged { word: Word("HI".into()), n: 1 }));
    println!("{}", tagged.contains(&Tagged { word: Word("HI".into()), n: 2 }));

    let mut lists: HashMap<Word, Vec<u32>> = HashMap::new();
    lists.entry(Word("k".into())).or_insert_with(Vec::new).push(1);
    lists.entry(Word("K".into())).and_modify(|v| v.push(2)).or_default().push(3);
    for list in lists.values_mut() {
        list.push(4);
    }
    let copy = lists.clone();
    lists.get_mut(&Word("K".into())).unwrap().push(5);
    println!("{lists:?} {copy:?}");
    let drained: Vec<(Word, Vec<u32>)> = lists.drain().collect();
    println!("{drained:?} {}", lists.len());

    let mut one: HashSet<Word> = HashSet::new();
    one.insert(Word("q".into()));
    let other = one.clone();
    println!("{other:?} {}", other == one);
    let back: Vec<Word> = one.into_iter().collect();
    println!("{back:?}");
}

fn panics() {
    let mut map: HashMap<Fragile, &str> = HashMap::new();
    map.insert(Fragile(1), "one");
    map.insert(Fragile(2), "two");
    PANIC_AT.with(|p| p.set(3));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        map.insert(Fragile(3), "three");
    }));
    PANIC_AT.with(|p| p.set(0));
    println!("{}", caught.is_err());
    let mut left: Vec<(u32, &str)> = map.iter().map(|(k, v)| (k.0, *v)).collect();
    left.sort();
    println!("{left:?}");

    let mut held = HashSet::from([Held { fragile: Fragile(1) }]);
    let mut options = HashSet::from([Some(Fragile(1)), None]);
    PANIC_AT.with(|p| p.set(3));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        held.insert(Held { fragile: Fragile(3) });
    }));
    println!("{}", caught.is_err());
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        options.insert(Some(Fragile(3)));
    }));
    println!("{}", caught.is_err());
    PANIC_AT.with(|p| p.set(0));
    println!("{} {}", held.len(), options.len());
}

fn main() {
    maps();
    sets();
    holding();
    panics();
}

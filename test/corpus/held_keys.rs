// A map or a set that finds a key by another gives back the one it holds:
// a path's, `a/b` found by `a//b` (ADR 0401), a `Cow`'s (ADR 0402), and a
// B-tree's of its own `Ord` (ADR 0404).
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Version(u32, &'static str);

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

fn main() {
    let mut files = HashSet::new();
    files.insert(PathBuf::from("src/lib.rs"));
    println!("{:?}", files.get(Path::new("src//lib.rs")));
    println!("{:?}", files.get(Path::new("src/main.rs")));
    println!("{:?}", files.replace(PathBuf::from("src/./lib.rs")));
    println!("{:?}", files.take(Path::new("src/lib.rs/")));
    println!("{}", files.len());
    let mut ids = HashSet::from([1]);
    println!("{:?} {:?}", ids.replace(2), ids.replace(1));

    let mut words: HashMap<Cow<str>, u32> = HashMap::new();
    words.insert(Cow::Owned("hi".to_string()), 1);
    println!("{:?}", words.get_key_value("hi"));
    let tags: HashSet<Cow<str>> = HashSet::from([Cow::Owned("new".to_string())]);
    println!("{}", matches!(tags.get("new"), Some(Cow::Owned(_))));

    let mut sizes = BTreeMap::new();
    sizes.insert(PathBuf::from("a/b"), 1);
    if let Entry::Occupied(entry) = sizes.entry(PathBuf::from("a//b")) {
        println!("{:?}", entry.key());
        println!("{:?}", entry.remove_entry());
    }

    let mut releases = BTreeSet::new();
    releases.insert(Version(2, "two"));
    println!("{:?}", releases.get(&Version(2, "")));
    println!("{:?}", releases.replace(Version(2, "deux")));
    println!("{:?}", releases.take(&Version(2, "")));
    println!("{:?}", releases.take(&Version(2, "")));
}

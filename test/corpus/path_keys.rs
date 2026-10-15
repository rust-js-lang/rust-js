// A map or a set of paths finds a path by its components, `a//b` the
// `a/b` put in, which it gives back as it was put in; a B-tree's are in
// their components' order (ADR 0401).
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

fn main() {
    let mut sizes: HashMap<PathBuf, u32> = HashMap::new();
    sizes.insert(PathBuf::from("src/main.rs"), 10);
    sizes.insert(PathBuf::from("src//lib.rs"), 20);
    println!("{:?}", sizes.get(Path::new("src/./main.rs")));
    println!("{:?}", sizes.insert(PathBuf::from("src/lib.rs"), 25));
    println!("{} {}", sizes.len(), sizes.contains_key(Path::new("src/lib.rs/")));
    let mut names: Vec<String> = sizes.keys().map(|p| p.display().to_string()).collect();
    names.sort();
    println!("{:?}", names);
    *sizes.entry(PathBuf::from("./src/main.rs")).or_insert(0) += 1;
    *sizes.entry(PathBuf::from("src/main.rs")).or_insert(0) += 1;
    println!("{:?} {:?}", sizes.get(Path::new("src/main.rs")), sizes.get(Path::new("./src/main.rs")));
    println!("{:?}", sizes.remove(Path::new("src//main.rs")));

    let mut seen: HashSet<&Path> = HashSet::new();
    for p in ["a/b", "a//b", "a/b/", "./a/b", "/a/b"] {
        println!("{} {}", p, seen.insert(Path::new(p)));
    }

    let mut tree: BTreeMap<PathBuf, u8> = BTreeMap::new();
    for (i, p) in ["a.b", "a/b", "a", "/z", "../up", "./here"].iter().enumerate() {
        tree.insert(PathBuf::from(p), i as u8);
    }
    println!("{:?}", tree);
    let set: BTreeSet<PathBuf> = ["b/c", "b", "b//c"].iter().map(PathBuf::from).collect();
    println!("{:?}", set);
    let from = BTreeMap::from([(PathBuf::from("x/y"), 1), (PathBuf::from("x//y"), 2)]);
    println!("{:?}", from);
    let hashed: HashSet<PathBuf> = ["q/r", "q//r"].iter().map(PathBuf::from).collect();
    println!("{:?}", hashed);
}

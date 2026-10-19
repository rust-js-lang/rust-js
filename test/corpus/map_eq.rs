// `==` of two maps and of two sets: as many of each, and each key of one
// the other's, of an equal value, whatever order they were made in (ADR
// 0059).
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

// Equal whatever its case.
#[derive(Debug, Clone)]
struct Name(String);

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Inventory {
    counts: HashMap<String, u32>,
    tags: BTreeSet<&'static str>,
}

fn main() {
    let a: HashMap<u32, String> = [(1, "x".to_string()), (2, "y".to_string())].into();
    let mut b: HashMap<u32, String> = [(2, "y".to_string()), (1, "x".to_string())].into();
    println!("{} {}", a == b, a != b);
    b.insert(2, "z".to_string());
    println!("{} {}", a == b, a != b);
    b.remove(&2);
    println!("{}", a == b);

    let s: HashSet<&str> = ["a", "b"].into();
    let t: HashSet<&str> = ["b", "a"].into();
    let u: HashSet<&str> = ["a"].into();
    println!("{} {}", s == t, s == u);

    let m: BTreeMap<u32, Vec<u32>> = [(1, vec![1, 2])].into();
    let n: BTreeMap<u32, Vec<u32>> = [(1, vec![1, 3])].into();
    println!("{} {}", m == m.clone(), m == n);

    let p: HashMap<Point, &str> = [(Point { x: 1, y: 2 }, "here")].into();
    let q: HashMap<Point, &str> = [(Point { x: 1, y: 2 }, "here")].into();
    println!("{}", p == q);

    let names: HashMap<u32, Name> = [(1, Name("Ann".into()))].into();
    let shouted: HashMap<u32, Name> = [(1, Name("ANN".into()))].into();
    println!("{}", names == shouted);

    let nan: HashMap<u32, f64> = [(1, f64::NAN)].into();
    println!("{}", nan == nan.clone());

    let one = Inventory { counts: [("apple".to_string(), 2)].into(), tags: ["red"].into() };
    let mut two = one.clone();
    println!("{}", one == two);
    two.counts.insert("pear".to_string(), 1);
    println!("{}", one == two);

    // A map of values with their own `==`, in a struct: each by its own.
    #[derive(PartialEq)]
    struct Roster {
        names: HashMap<u32, Name>,
    }
    let left = Roster { names: names.clone() };
    let right = Roster { names: shouted.clone() };
    println!("{}", left == right);
}

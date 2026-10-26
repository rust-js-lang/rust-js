// A map or a set keyed by an `Option` finds a `None` key whichever of JS's
// two it is (ADRs 0121, 0935): a `#[rust_js::nullable]` field's `None` is
// `null`, a variable's `undefined`, and Rust has one `None`.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

struct Node {
    #[cfg_attr(rust_js, rust_js::nullable)]
    parent: Option<u32>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    label: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    mark: Option<char>,
}

fn root() -> Node {
    Node {
        parent: None,
        label: None,
        mark: None,
    }
}

fn main() {
    let node = root();

    let mut by_parent: HashMap<Option<u32>, &str> = HashMap::new();
    by_parent.insert(node.parent, "root");
    by_parent.insert(None, "other root");
    println!("{} {:?}", by_parent.len(), by_parent.get(&None));

    let mut labels: HashSet<Option<String>> = HashSet::new();
    labels.insert(node.label.clone());
    println!("{} {}", labels.insert(None), labels.contains(&None));

    let mut marks: HashMap<Option<char>, u32> = HashMap::new();
    *marks.entry(node.mark).or_insert(0) += 1;
    *marks.entry(None).or_insert(0) += 1;
    println!("{:?}", marks.get(&None));

    let mut levels: BTreeMap<Option<u32>, u32> = BTreeMap::new();
    levels.insert(Some(2), 2);
    levels.insert(node.parent, 0);
    levels.insert(None, 1);
    println!("{:?}", levels);

    let mut sorted: BTreeSet<Option<String>> = BTreeSet::new();
    sorted.insert(Some("a".to_string()));
    sorted.insert(node.label.clone());
    sorted.insert(None);
    println!("{:?}", sorted);
}

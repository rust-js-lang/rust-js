use indexmap::{IndexMap, IndexSet};

pub fn report() -> String {
    let mut lines = Vec::new();

    // Kept in the order added, as native indexmap keeps it.
    let mut ages: IndexMap<&str, u32> = IndexMap::new();
    ages.insert("carol", 41);
    ages.insert("alice", 30);
    ages.insert("bob", 25);
    ages.insert("alice", 31);
    lines.push(format!("{ages:?} {}", ages.len()));
    lines.push(format!(
        "{:?} {:?} {}",
        ages.get("bob"),
        ages.get("zed"),
        ages.contains_key("carol")
    ));
    *ages.entry("dave").or_insert(0) += 50;
    *ages.entry("bob").or_insert(0) += 1;
    let keys: Vec<_> = ages.keys().collect();
    lines.push(format!("{keys:?} {:?}", ages.values().sum::<u32>()));
    lines.push(format!(
        "{:?} {:?} {:?}",
        ages.get_index(1),
        ages.get_index_of("bob"),
        ages.first()
    ));
    let (index, old) = ages.insert_full("erin", 22);
    lines.push(format!("{index} {old:?} {:?}", ages.last()));

    // Removed: shifting what's after it, or swapping the last into its place.
    let mut shifted = ages.clone();
    lines.push(format!("{:?} {shifted:?}", shifted.shift_remove("alice")));
    let mut removed = ages.clone();
    #[allow(deprecated)]
    let gone = removed.remove("carol");
    lines.push(format!("{gone:?} {removed:?}"));
    lines.push(format!("{:?} {ages:?}", ages.swap_remove("alice")));
    lines.push(format!("{:?} {ages:?}", ages.pop()));
    ages.sort_keys();
    lines.push(format!("{ages:?}"));

    let mut tags: IndexSet<String> = IndexSet::new();
    for t in ["b", "a", "b", "c"] {
        tags.insert(t.to_string());
    }
    lines.push(format!("{tags:?} {:?} {}", tags.get_index_of("c"), tags.contains("a")));
    // Keyed by value: where a key is, of what it's kept as.
    let mut points: IndexMap<(u8, u8), &str> = IndexMap::new();
    points.insert((2, 1), "b");
    points.insert((0, 0), "o");
    lines.push(format!(
        "{:?} {:?}",
        points.get_index_of(&(0, 0)),
        points.swap_remove(&(2, 1))
    ));
    let collected: IndexMap<u32, char> = [(3, 'c'), (1, 'a')].into_iter().collect();
    lines.push(format!("{collected:?}"));

    lines.join("\n")
}

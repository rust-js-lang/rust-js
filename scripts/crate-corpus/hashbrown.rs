use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};

/// A map's iterator, held.
struct Walk<'a> {
    values: hashbrown::hash_map::Values<'a, String, u32>,
}

pub fn report() -> String {
    let mut lines = Vec::new();

    let mut counts: HashMap<String, u32> = HashMap::new();
    for word in "a b a c b a".split(' ') {
        *counts.entry(word.to_string()).or_insert(0) += 1;
    }
    let mut sorted: Vec<_> = counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
    sorted.sort();
    lines.push(format!("{} {:?}", counts.len(), sorted));
    lines.push(format!(
        "{:?} {:?} {}",
        counts.get("a"),
        counts.get("z"),
        counts.contains_key("b")
    ));
    if let Some(n) = counts.get_mut("c") {
        *n += 10;
    }
    let removed = counts.remove("b");
    lines.push(format!("{removed:?} {:?} {}", counts.get("c"), counts.len()));
    match counts.entry("d".to_string()) {
        Entry::Occupied(_) => lines.push("occupied".to_string()),
        Entry::Vacant(v) => {
            v.insert(4);
        }
    }
    let walk = Walk {
        values: counts.values(),
    };
    lines.push(format!("{}", walk.values.copied().max().unwrap_or(0)));
    let mut keys: Vec<&String> = counts.keys().collect();
    keys.sort();
    lines.push(format!("{keys:?} {}", counts.values().sum::<u32>()));

    let mut seen: HashSet<u32> = HashSet::new();
    let fresh: Vec<bool> = [3, 1, 3, 2, 1].iter().map(|n| seen.insert(*n)).collect();
    let mut items: Vec<_> = seen.iter().copied().collect();
    items.sort();
    lines.push(format!("{fresh:?} {items:?} {}", seen.contains(&2)));
    let other: HashSet<u32> = [2, 5].into_iter().collect();
    let mut both: Vec<_> = seen.intersection(&other).collect();
    both.sort();
    lines.push(format!("{both:?}"));

    lines.join("\n")
}

// A map or a set of `Cow`s finds one by what it borrows: `Borrowed("a")`,
// `Owned("a")` and `"a"` alike, the key it holds the first put in; a
// B-tree made of items keeps the last of equal ones, as std's does
// (ADR 0402).
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn kind(c: &Cow<'_, str>) -> &'static str {
    match c {
        Cow::Borrowed(_) => "borrowed",
        Cow::Owned(_) => "owned",
    }
}

fn main() {
    let mut counts: HashMap<Cow<'static, str>, u32> = HashMap::new();
    counts.insert(Cow::Borrowed("a"), 1);
    println!("{:?}", counts.insert(Cow::Owned("a".to_string()), 2));
    println!("{:?} {:?}", counts.get("a"), counts.get(&Cow::Borrowed("a")));
    *counts.entry(Cow::Owned("b".to_string())).or_insert(0) += 5;
    *counts.entry(Cow::Borrowed("b")).or_insert(0) += 5;
    println!("{} {}", counts.contains_key("b"), counts.len());
    let mut held: Vec<String> = counts.iter().map(|(k, v)| format!("{}={} {}", k, v, kind(k))).collect();
    held.sort();
    println!("{:?}", held);
    println!("{:?} {}", counts.remove("a"), counts.len());

    let mut seen: HashSet<Cow<'static, str>> = HashSet::new();
    println!("{} {}", seen.insert(Cow::Borrowed("x")), seen.insert(Cow::Owned("x".to_string())));

    let mut tree: BTreeMap<Cow<'static, str>, u8> = BTreeMap::new();
    tree.insert(Cow::Owned("pear".to_string()), 1);
    tree.insert(Cow::Borrowed("apple"), 2);
    tree.insert(Cow::Borrowed("pear"), 3);
    for (k, v) in &tree {
        println!("{} {} {}", k, v, kind(k));
    }
    let made: BTreeSet<Cow<'static, str>> = [Cow::Borrowed("z"), Cow::Owned("z".to_string())].into_iter().collect();
    println!("{:?}", made.iter().map(kind).collect::<Vec<_>>());

    let mut rows: HashMap<Cow<'static, [i32]>, &str> = HashMap::new();
    rows.insert(Cow::Borrowed(&[1, 2]), "first");
    rows.insert(Cow::Owned(vec![1, 2]), "second");
    println!("{:?} {}", rows.get(&[1, 2][..]), rows.len());
}

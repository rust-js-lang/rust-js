// A B-tree's `range_mut`: its keys in a range, in order, each with a `&mut`
// to its value, which writes the map.
use std::collections::BTreeMap;

fn main() {
    let mut scores: BTreeMap<u32, i32> = (1..=6).map(|k| (k, k as i32 * 10)).collect();
    for (k, v) in scores.range_mut(2..5) {
        *v += *k as i32;
    }
    println!("{scores:?}");
    let total: i32 = scores
        .range_mut(..=3)
        .map(|(_, v)| {
            *v *= 2;
            *v
        })
        .sum();
    println!("{total} {scores:?}");

    let mut names: BTreeMap<&str, String> =
        [("ann", "a".to_string()), ("bob", "b".to_string()), ("cy", "c".to_string())].into();
    for (_, name) in names.range_mut("b"..) {
        name.push('!');
    }
    if let Some((k, v)) = names.range_mut(..="ann").last() {
        v.insert(0, '>');
        println!("{k}");
    }
    let mut last = names.range_mut("a".."c");
    if let Some((_, v)) = last.next() {
        *v = v.to_uppercase();
    }
    println!("{names:?} {}", last.count());

    let mut points: BTreeMap<u8, (i32, i32)> = (0..4).map(|k| (k, (k as i32, 0))).collect();
    for (_, p) in points.range_mut(1..3) {
        p.1 += 5;
    }
    println!("{points:?} {}", points.range_mut(9..).count());

    let set: std::collections::BTreeSet<u8> = (0..5).collect();
    for x in set.range(2..4) {
        print!("{x} ");
    }
    println!();
}

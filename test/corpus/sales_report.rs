// A program's data processing: records grouped into a `BTreeMap`, sorted
// by keys, summed, `windows`, `scan`, `step_by`, `zip` with `cycle`, `chunks`,
// `dedup` and `max_by`, each as native Rust gives it (ADRs 0036, 0139).
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
struct Sale {
    region: String,
    item: &'static str,
    units: u32,
    price: f64,
}

fn sales() -> Vec<Sale> {
    let raw = [
        ("north", "pen", 3, 1.5),
        ("south", "ink", 1, 4.25),
        ("north", "ink", 2, 4.0),
        ("east", "pad", 7, 2.0),
        ("south", "pen", 5, 1.25),
    ];
    raw.iter()
        .map(|&(r, item, units, price)| Sale {
            region: r.to_string(),
            item,
            units,
            price,
        })
        .collect()
}

fn main() {
    let all = sales();
    let mut by_region: BTreeMap<&str, Vec<&Sale>> = BTreeMap::new();
    for s in &all {
        by_region.entry(s.region.as_str()).or_default().push(s);
    }
    for (region, list) in &by_region {
        let revenue: f64 = list.iter().map(|s| s.units as f64 * s.price).sum();
        println!("{region}: {} sales, {revenue:.2}", list.len());
    }
    let mut sorted = all.clone();
    sorted.sort_by(|a, b| b.units.cmp(&a.units).then_with(|| a.item.cmp(b.item)));
    let top: Vec<String> = sorted
        .iter()
        .take(3)
        .map(|s| format!("{}x{}", s.units, s.item))
        .collect();
    println!("{}", top.join(", "));
    let items: HashSet<&str> = all.iter().map(|s| s.item).collect();
    let mut items: Vec<_> = items.into_iter().collect();
    items.sort_unstable();
    println!("{items:?}");
    let units: Vec<u32> = all.iter().map(|s| s.units).collect();
    let diffs: Vec<i64> = units.windows(2).map(|w| w[1] as i64 - w[0] as i64).collect();
    println!("{diffs:?}");
    let running: Vec<u32> = units
        .iter()
        .scan(0, |acc, &u| {
            *acc += u;
            Some(*acc)
        })
        .collect();
    println!("{running:?}");
    let pairs: Vec<(usize, &str)> = all.iter().map(|s| s.item).enumerate().step_by(2).collect();
    println!("{pairs:?}");
    let zipped: Vec<String> = units
        .iter()
        .zip(items.iter().cycle())
        .map(|(u, i)| format!("{u}{i}"))
        .collect();
    println!("{zipped:?}");
    let chunks: Vec<u32> = units.chunks(2).map(|c| c.iter().product()).collect();
    println!("{chunks:?}");
    let letters: String = items.iter().rev().flat_map(|i| i.chars().take(1)).collect();
    println!("{letters}");
    let mut regions: Vec<&str> = all.iter().map(|s| s.region.as_str()).collect();
    regions.sort();
    regions.dedup();
    println!("{regions:?} {}", regions.contains(&"east"));
    let max = all
        .iter()
        .max_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
        .unwrap();
    println!("{:?}", max);
    println!(
        "{}",
        all.iter().any(|s| s.units > 6) && all.iter().all(|s| s.price > 1.0)
    );
}

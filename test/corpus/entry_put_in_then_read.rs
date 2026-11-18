// An entry's value put in where it's missing, then read whole, `*cache
// .entry(key).or_insert_with(|| resolve(key))`: the map's own steps, `if
// (!cache.has(key)) { cache.set(key, resolve(key)); } return
// cache.get(key);`, its maker run only where the key is missing, as the
// vercel-ui SDK's memo of a locale's first day is (G3). Not where
// something runs between the put and the read.

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

fn resolve(key: &str) -> f64 {
    println!("resolve {key}");
    key.len() as f64
}

fn memo(locale: Option<&str>) -> f64 {
    let key = locale.unwrap_or("");
    CACHE.with_borrow_mut(|cache| *cache.entry(key.to_string()).or_insert_with(|| resolve(key)))
}

fn counted(counts: &mut HashMap<char, u32>, c: char) -> u32 {
    *counts.entry(c).or_insert(0)
}

fn sized(sizes: &mut HashMap<u32, usize>, k: u32) -> usize {
    let len = sizes.len();
    *sizes.entry(k).or_insert_with(|| len * 10)
}

fn noted(seen: &mut HashMap<u32, u32>, k: u32, log: &mut Vec<u32>) -> u32 {
    let value = seen.entry(k).or_insert_with(|| k * 2);
    log.push(k);
    *value
}

fn main() {
    for locale in [Some("de-DE"), None, Some("de-DE"), Some("en")] {
        println!("{}", memo(locale));
    }
    let mut counts = HashMap::from([('a', 3)]);
    println!("{} {}", counted(&mut counts, 'a'), counted(&mut counts, 'b'));
    let mut sizes = HashMap::new();
    println!(
        "{} {} {}",
        sized(&mut sizes, 7),
        sized(&mut sizes, 8),
        sized(&mut sizes, 7)
    );
    let mut seen = HashMap::new();
    let mut log = Vec::new();
    println!(
        "{} {} {:?}",
        noted(&mut seen, 4, &mut log),
        noted(&mut seen, 4, &mut log),
        log
    );
}

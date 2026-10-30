//@ library-differs
// `collect()` into a `Result` or an `Option` of a collection that isn't
// an array, a map, a set, text or one of the crate's own: the first `Err`
// or `None`, or all the values made into it, `$collectResults(items,
// (values) => new Map(values))`, `Bag`'s by its own `from_iter`.
use std::collections::{BTreeMap, BTreeSet, HashMap};
fn parse_kv(s: &str) -> Result<BTreeMap<String, i64>, String> {
    s.split(';').filter(|p| !p.is_empty()).map(|pair| {
        let (k, v) = pair.split_once('=').ok_or_else(|| format!("no = in {pair:?}"))?;
        let v: i64 = v.parse().map_err(|e| format!("{k}: {e}"))?;
        Ok((k.to_string(), v))
    }).collect()
}
#[derive(Debug)]
struct Bag(Vec<i32>);

impl FromIterator<i32> for Bag {
    fn from_iter<I: IntoIterator<Item = i32>>(it: I) -> Self {
        let mut v: Vec<i32> = it.into_iter().collect();
        v.sort();
        Bag(v)
    }
}

fn bag(words: &[&str]) -> Result<Bag, String> {
    words.iter().map(|w| w.parse::<i32>().map_err(|e| format!("{w}: {e}"))).collect()
}

fn main() {
    let some: Option<Bag> = [Some(5), Some(4)].into_iter().collect();
    let none: Option<Bag> = [Some(5), None].into_iter().collect();
    println!("{:?} {:?} {some:?} {none:?}", bag(&["3", "1", "2"]), bag(&["1", "x"]));
    println!("{:?} {:?}", parse_kv("a=1;b=2;"), parse_kv("a=1;b=2").map(|m| m.get("b").copied()));
    println!("{:?}", parse_kv("a=1;b"));
    let words: Result<String, String> = ["ab", "cd"].iter().map(|w| if w.len() == 2 { Ok(w.to_uppercase()) } else { Err(w.to_string()) }).collect();
    let set: Option<BTreeSet<u32>> = ["3", "1", "3"].iter().map(|s| s.parse().ok()).collect();
    let none: Option<BTreeSet<u32>> = ["3", "x"].iter().map(|s| s.parse().ok()).collect();
    let m: Result<HashMap<u8, u8>, ()> = (1..4).map(|i| Ok((i, i * i))).collect();
    let mut pairs: Vec<_> = m.unwrap().into_iter().collect();
    pairs.sort();
    let vec_ok: Result<Vec<i32>, String> = ["1", "2"].iter().map(|s| s.parse::<i32>().map_err(|e| e.to_string())).collect();
    println!("{words:?} {set:?} {none:?} {pairs:?} {vec_ok:?}");
}

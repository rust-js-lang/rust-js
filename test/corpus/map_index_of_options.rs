// `m[k]` of a map whose values could look like `None`: a `None` that's
// there is read, not taken for a key that isn't, its `Some` boxed as
// `get`'s is, as Gale-Shapley's `current_partner[woman]` reads a partner.
use std::collections::{BTreeMap, HashMap};

fn main() {
    let mut partner: HashMap<String, Option<String>> = HashMap::new();
    partner.insert("ann".to_string(), None);
    partner.insert("bea".to_string(), Some("cal".to_string()));
    println!("{:?} {:?}", partner["ann"], partner["bea"]);
    if let Some(current) = partner["bea"].clone() {
        println!("bea is with {current}");
    }

    let nested: BTreeMap<u8, Option<Option<u8>>> =
        [(1, None), (2, Some(None)), (3, Some(Some(4)))].into_iter().collect();
    println!("{:?} {:?} {:?}", nested[&1], nested[&2], nested[&3]);
}

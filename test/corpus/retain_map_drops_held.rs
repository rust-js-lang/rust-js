// `retain` of a map whose values are given to its closure by handle, a
// `Box`'s, an `Rc`'s, an `Option`'s, drops each value it doesn't keep, not
// the handle (#905, ADR 0321).
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut boxes: HashMap<u32, Box<Loud>> = HashMap::new();
    boxes.insert(1, Box::new(Loud(1)));
    boxes.retain(|_, x| x.0 % 2 == 0);
    println!("boxes {}", boxes.len());

    let mut shared: HashMap<u32, Rc<Loud>> = HashMap::new();
    shared.insert(1, Rc::new(Loud(2)));
    shared.retain(|_, x| x.0 % 2 == 1);
    println!("shared {}", shared.len());

    let mut options: HashMap<u32, Option<Loud>> = HashMap::new();
    options.insert(1, Some(Loud(3)));
    options.retain(|_, x| x.is_none());
    println!("options {}", options.len());

    // Changed through its handle first: what it holds then is dropped.
    let mut sorted: BTreeMap<u32, Box<Loud>> = BTreeMap::new();
    sorted.insert(1, Box::new(Loud(4)));
    sorted.insert(2, Box::new(Loud(5)));
    sorted.retain(|k, x| {
        **x = Loud(x.0 + 10);
        k % 2 == 0
    });
    println!("sorted {}", sorted.len());
}

// A map lent where a generic `&mut T` goes is given in a box, and what the
// box holds after is the map: still the same map, with what was added.
use std::collections::HashMap;

trait Grow {
    fn grow(&mut self);
}

impl Grow for HashMap<u32, u32> {
    fn grow(&mut self) {
        let n = self.len() as u32;
        self.insert(n, n * 10);
    }
}

fn twice<T: Grow>(t: &mut T) {
    t.grow();
    t.grow();
}

fn main() {
    let mut m = HashMap::new();
    m.insert(7, 70);
    twice(&mut m);
    let mut pairs: Vec<(u32, u32)> = m.iter().map(|(k, v)| (*k, *v)).collect();
    pairs.sort();
    println!("{pairs:?}");
}

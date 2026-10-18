// What a method passes over of items with a destructor it drops, as std's
// does: `nth`'s and `find`'s, of a known iterator and a generic one, and
// `partition` keeps each in one of its two (ADRs 0098, 0446).

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn third<I: Iterator<Item = Noisy>>(mut it: I) -> Option<Noisy> {
    it.nth(2)
}

fn named<I: Iterator<Item = Noisy>>(mut it: I, name: &str) -> Option<Noisy> {
    it.find(|n| n.0 == name)
}

fn main() {
    let v = vec![Noisy("a"), Noisy("b"), Noisy("c"), Noisy("d")];
    let mut it = v.into_iter();
    let x = it.nth(1);
    println!("got {}", x.as_ref().map(|n| n.0).unwrap_or("none"));
    drop(x);
    let y = third(vec![Noisy("p"), Noisy("q"), Noisy("r")].into_iter());
    println!("got {}", y.as_ref().map(|n| n.0).unwrap_or("none"));
    let z = third(vec![Noisy("s")].into_iter());
    println!("none {}", z.is_none());
    let f = vec![Noisy("e"), Noisy("f"), Noisy("g")].into_iter().find(|n| n.0 == "f");
    println!("found {}", f.as_ref().map(|n| n.0).unwrap_or("none"));
    let g = named(vec![Noisy("h"), Noisy("i")].into_iter(), "i");
    println!("found {}", g.as_ref().map(|n| n.0).unwrap_or("none"));
    let (kept, rest): (Vec<Noisy>, Vec<Noisy>) =
        vec![Noisy("j"), Noisy("kk"), Noisy("l")].into_iter().partition(|n| n.0.len() == 1);
    println!("{} {}", kept.len(), rest.len());
    println!("end");
}

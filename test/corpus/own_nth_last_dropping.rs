// Items with a destructor through generic code's `nth` and `last` (#182):
// the crate's own `nth` and `last`, and a `fuse`'s, std's compiled body,
// which drop what they pass themselves; an iterator without them, each it
// passes dropped as std's default drops it.
struct Noisy(u32);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Own {
    at: u32,
}
impl Iterator for Own {
    type Item = Noisy;
    fn next(&mut self) -> Option<Noisy> {
        (self.at < 5).then(|| {
            self.at += 1;
            Noisy(self.at)
        })
    }
    fn nth(&mut self, n: usize) -> Option<Noisy> {
        println!("own nth {n}");
        self.at += n as u32;
        self.next()
    }
    fn last(self) -> Option<Noisy> {
        println!("own last");
        Some(Noisy(99))
    }
}
struct Plain {
    at: u32,
}
impl Iterator for Plain {
    type Item = Noisy;
    fn next(&mut self) -> Option<Noisy> {
        (self.at < 5).then(|| {
            self.at += 1;
            Noisy(self.at)
        })
    }
}
fn third<I: Iterator<Item = Noisy>>(mut it: I) -> Option<u32> {
    it.nth(2).map(|n| n.0)
}
fn final_one<I: Iterator<Item = Noisy>>(it: I) -> Option<u32> {
    it.last().map(|n| n.0)
}
fn main() {
    println!("{:?}", third(Own { at: 0 }));
    println!("{:?}", third(Plain { at: 0 }));
    println!("{:?}", third(Plain { at: 0 }.map(|n| Noisy(n.0 * 10))));
    println!("{:?}", final_one(Own { at: 0 }));
    println!("{:?}", final_one(Plain { at: 0 }));
    println!("{:?}", final_one(Own { at: 0 }.fuse()));
}

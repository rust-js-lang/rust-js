// A flatten of the crate's iterators, stepped by `nth` or `skip(k).next()`,
// in one statement and held, drops each inner iterator it runs out as std's
// `Flatten` does, in `Option::insert` of the next, which generic code drops
// by its `IntoIterator` dictionary's `$dropIntoIter`. Std's `IntoIterator`
// of a type known here had none, so the std build never dropped them (#1510).
struct C {
    at: u32,
    end: u32,
    tag: &'static str,
}
impl Iterator for C {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.at < self.end).then(|| {
            self.at += 1;
            self.at - 1
        })
    }
}
impl Drop for C {
    fn drop(&mut self) {
        println!("drop {}", self.tag);
    }
}
fn c(tag: &'static str) -> C {
    C { at: 0, end: 3, tag }
}
fn main() {
    println!("{:?}", vec![c("a1"), c("a2")].into_iter().flatten().skip(4).next());
    println!("{:?}", vec![c("b1"), c("b2")].into_iter().flatten().nth(4));
    let mut h = vec![c("h1"), c("h2")].into_iter().flatten();
    println!("{:?}", h.nth(4));
    println!("held");
    drop(h);
}

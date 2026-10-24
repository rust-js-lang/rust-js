// The crate's iterator's own `nth` and `last`, which std passes on through
// its adapters (#182): `nth` through a `fuse` and a `peekable`, std's
// compiled `nth`, and `skip(n).any(..)`'s `nth(n - 1)` behind them; `last`
// through a `skip`, after its `nth(n - 1)`, a `fuse`, a `peekable`, and a
// chain, each side's.
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
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("{} nth {n}", self.tag);
        self.at += n as u32;
        self.next()
    }
    fn last(self) -> Option<u32> {
        println!("{} own last", self.tag);
        (self.at < self.end).then_some(self.end - 1)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("{} hint", self.tag);
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
}
fn c(tag: &'static str) -> C {
    C { at: 0, end: 6, tag }
}
fn main() {
    println!("{:?}", c("fuse").fuse().nth(2));
    println!("{:?}", c("peek").peekable().nth(2));
    println!("{:?}", c("skiplast").skip(1).last());
    println!("{:?}", c("fuselast").fuse().last());
    println!("{:?}", c("peeklast").peekable().last());
    println!("{:?}", c("chainlast").chain(c("b")).last());
    println!("{:?}", c("fuseany").fuse().skip(2).any(|x| x == 4));
    println!("{:?}", c("peekany").peekable().skip(2).any(|x| x == 4));
    println!(
        "{:?}",
        vec![c("flat1"), c("flat2")].into_iter().flatten().skip(7).next()
    );
}
